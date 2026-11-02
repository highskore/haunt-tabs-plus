//! Haunt's tab arranging as a plugin: manual order, pins, custom groups and colours, each one a
//! setting. State lives in kv; the sidebar is arranged through the host's `sidebar` import.
// wit-bindgen's flattened `notify` import has 14 params.
#![allow(clippy::too_many_arguments)]

wit_bindgen::generate!({ path: "wit/haunt-plugin.wit", world: "plugin" });

mod tabs;

use std::sync::Mutex;

use haunt::plugin::log::log;
use haunt::plugin::sidebar::{self, RowInfo};
use haunt::plugin::types::LogLevel;
use haunt::plugin::{kv, palette};
use tabs::{COLORS, Settings, State, valid_name};

const KEY: &str = "state";
const GROUP_MODE: &str = "group";
const COLOR_MODE: &str = "color";

static SETTINGS: Mutex<Option<Settings>> = Mutex::new(None);

struct Plugin;

fn settings() -> Settings {
    SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default()
}

fn load() -> State {
    kv::get(KEY).map(|j| State::parse(&j)).unwrap_or_default()
}

/// Saves `st` and pushes its arrangement over the current rows.
fn commit(mut st: State, rows: &[RowInfo]) {
    st.prune(rows);
    if let Err(e) = kv::set(KEY, &st.to_json()) {
        log(LogLevel::Warn, &format!("tabs: state not saved: {e}"));
    }
    if let Err(e) = sidebar::arrange(&st.arrangement(&settings(), rows)) {
        log(LogLevel::Warn, &format!("tabs: arrange refused: {e}"));
    }
}

/// The row an action is about: the menu's row, else the selected tab.
fn target(rows: &[RowInfo], payload: &str) -> Option<String> {
    let s = if payload.is_empty() { rows.iter().find(|r| r.selected)?.surface.clone() } else { payload.to_owned() };
    rows.iter().any(|r| r.surface == s).then_some(s)
}

fn item(id: String, title: String, subtitle: Option<String>) -> PaletteItem {
    PaletteItem { id, title, subtitle, at_ms: None, mark: None, surface: None }
}

fn entry(id: &str, title: &str, checked: bool, parent: Option<&str>) -> MenuItem {
    MenuItem { id: id.into(), title: title.into(), checked, parent: parent.map(Into::into) }
}

/// Runs a palette or menu action on `surface`; true if the state changed.
fn apply(st: &mut State, s: &Settings, rows: &[RowInfo], action: &str, surface: &str) -> bool {
    match action {
        "pin" if s.pins => st.toggle_pin(surface),
        "move-up" => return st.move_by(s, rows, surface, -1),
        "move-down" => return st.move_by(s, rows, surface, 1),
        "ungroup" if s.custom_groups => st.set_group(surface, None),
        "color:none" if s.colors => st.set_color(surface, None),
        a => {
            if let Some(g) = a.strip_prefix("group:").filter(|_| s.custom_groups) {
                st.set_group(surface, Some(g));
            } else if let Some(name) = a.strip_prefix("new:").filter(|_| s.custom_groups) {
                let Some(id) = st.new_group(name) else { return false };
                st.set_group(surface, Some(&id));
            } else if let Some(g) = a.strip_prefix("delete:").filter(|_| s.custom_groups) {
                st.delete_group(g);
            } else if let Some(c) = a.strip_prefix("color:").filter(|_| s.colors) {
                st.set_color(surface, Some(c));
            } else {
                return false;
            }
        }
    }
    true
}

impl Guest for Plugin {
    fn init(json: String) {
        let s = Settings::parse(&json);
        if s.pins {
            palette::command("pin", "Pin or unpin tab", None);
        }
        if s.manual_order {
            palette::command("move-up", "Move tab up", Some("alt+cmd+up"));
            palette::command("move-down", "Move tab down", Some("alt+cmd+down"));
        }
        if s.custom_groups {
            palette::mode(GROUP_MODE, "Put the tab in a group");
        }
        if s.colors {
            palette::mode(COLOR_MODE, "Colour the tab");
        }
        *SETTINGS.lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
        commit(load(), &sidebar::rows());
    }

    /// Palette entries registered at `init` stay until the next load.
    fn on_settings_changed(json: String) {
        *SETTINGS.lock().unwrap_or_else(|e| e.into_inner()) = Some(Settings::parse(&json));
        commit(load(), &sidebar::rows());
    }

    fn on_event(_event: AgentEvent) -> Option<Decoration> {
        None
    }

    fn palette_items(mode: String, query: String) -> Vec<PaletteItem> {
        let (s, st) = (settings(), load());
        let q = query.trim().to_lowercase();
        let hit = |t: &str| q.is_empty() || t.to_lowercase().contains(&q);
        match mode.as_str() {
            GROUP_MODE if s.custom_groups => {
                let mut out: Vec<_> =
                    st.groups.iter().filter(|g| hit(&g.name)).map(|g| item(format!("group:{}", g.id), format!("Move to {}", g.name), None)).collect();
                if let Some(name) = valid_name(&query).filter(|n| !st.groups.iter().any(|g| g.name.eq_ignore_ascii_case(n))) {
                    out.push(item(format!("new:{name}"), format!("New group “{name}”"), None));
                }
                out.push(item("ungroup".into(), "Remove from its group".into(), None));
                out.extend(st.groups.iter().filter(|g| hit(&g.name)).map(|g| item(format!("delete:{}", g.id), format!("Delete group {}", g.name), Some("its tabs go back to their state groups".into()))));
                out
            }
            COLOR_MODE if s.colors => COLORS
                .iter()
                .filter(|(_, t, _)| hit(t))
                .map(|(k, t, _)| item(format!("color:{k}"), (*t).into(), None))
                .chain(hit("none").then(|| item("color:none".into(), "No colour".into(), None)))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn on_action(action_id: String, payload: String) {
        let rows = sidebar::rows();
        let Some(surface) = target(&rows, &payload) else { return };
        let mut st = load();
        if apply(&mut st, &settings(), &rows, &action_id, &surface) {
            commit(st, &rows);
        }
    }

    fn row_menu(surface: String) -> Vec<MenuItem> {
        let (s, st) = (settings(), load());
        let mut out = Vec::new();
        if s.pins {
            out.push(entry("pin", "Pin", st.pinned.contains(&surface), None));
        }
        if s.manual_order {
            out.push(entry("move-up", "Move up", false, None));
            out.push(entry("move-down", "Move down", false, None));
        }
        if s.custom_groups && !st.groups.is_empty() {
            let current = st.member.get(&surface);
            out.push(entry("group-menu", "Group", false, None));
            for g in &st.groups {
                out.push(entry(&format!("group:{}", g.id), &g.name, current == Some(&g.id), Some("group-menu")));
            }
            out.push(entry("ungroup", "None", current.is_none(), Some("group-menu")));
        }
        if s.colors {
            let current = st.colors.get(&surface).map(String::as_str);
            out.push(entry("color-menu", "Colour", false, None));
            for (k, t, _) in COLORS {
                out.push(entry(&format!("color:{k}"), t, current == Some(k), Some("color-menu")));
            }
            out.push(entry("color:none", "None", current.is_none(), Some("color-menu")));
        }
        out
    }

    fn on_row_moved(surface: String, group: String, before: Option<String>) {
        let rows = sidebar::rows();
        let mut st = load();
        if st.move_to(&settings(), &rows, &surface, &group, before.as_deref()) {
            commit(st, &rows);
        }
    }
}

export!(Plugin);

#[cfg(test)]
mod tests {
    use super::*;
    use haunt::plugin::sidebar::StateGroup;

    fn rows() -> Vec<RowInfo> {
        ["a", "b"].iter().map(|s| RowInfo { surface: (*s).into(), name: (*s).into(), kind: "shell".into(), group: StateGroup::Idle, selected: *s == "b" }).collect()
    }

    #[test]
    fn actions_go_to_the_menu_row_or_the_selected_tab() {
        assert_eq!(target(&rows(), "a").as_deref(), Some("a"));
        assert_eq!(target(&rows(), "").as_deref(), Some("b"));
        assert_eq!(target(&rows(), "gone"), None);
    }

    #[test]
    fn actions_respect_disabled_features() {
        let rows = rows();
        let mut st = State::default();
        let off = Settings { pins: false, colors: false, custom_groups: false, ..Settings::default() };
        for a in ["pin", "color:red", "new:Work", "group:x"] {
            assert!(!apply(&mut st, &off, &rows, a, "a"), "{a}");
        }
        assert_eq!(st, State::default());
        let on = Settings::default();
        assert!(apply(&mut st, &on, &rows, "new:Work", "a"));
        assert!(apply(&mut st, &on, &rows, "color:red", "a"));
        assert_eq!((st.member.get("a").map(String::as_str), st.colors.get("a").map(String::as_str)), (Some("work"), Some("red")));
        assert!(!apply(&mut st, &on, &rows, "new:needs you", "b"), "a core group's name");
    }

    #[test]
    fn manifest_asks_for_the_sidebar_only_and_defaults_match() {
        let m = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/haunt-plugin.toml")).unwrap();
        let t: toml::Table = toml::from_str(&m).unwrap();
        assert_eq!(t["capabilities"].as_table().unwrap().keys().collect::<Vec<_>>(), ["sidebar"]);
        let defaults: serde_json::Map<String, serde_json::Value> = t["settings"]
            .as_table()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::to_value(&v["default"]).unwrap()))
            .collect();
        assert_eq!(Settings::parse(&serde_json::Value::Object(defaults).to_string()), Settings::default());
    }
}
