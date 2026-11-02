//! The tab state (manual order, pins, custom groups, colours) and how it becomes an arrangement.
//! Pure, so it is tested natively; `lib.rs` wires it to kv and the host.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::haunt::plugin::sidebar::{Arrangement, ColorStyle, Placement, RowInfo, Section, StateGroup, TabColor};

pub const COLORS: [(&str, &str, TabColor); 7] = [
    ("red", "Red", TabColor::Red),
    ("orange", "Orange", TabColor::Orange),
    ("yellow", "Yellow", TabColor::Yellow),
    ("green", "Green", TabColor::Green),
    ("blue", "Blue", TabColor::Blue),
    ("purple", "Purple", TabColor::Purple),
    ("gray", "Gray", TabColor::Gray),
];
const MAX_GROUPS: usize = 32;
const MAX_NAME: usize = 32;
const CORE_GROUPS: [&str; 5] = ["NEEDS YOU", "RUNNING", "IDLE", "PINNED", "TABS"];

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub manual_order: bool,
    pub flat: bool,
    pub custom_groups: bool,
    pub groups_placement: String,
    pub pins: bool,
    pub colors: bool,
    pub color_style: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            manual_order: true,
            flat: false,
            custom_groups: true,
            groups_placement: "before".into(),
            pins: true,
            colors: true,
            color_style: "dot".into(),
        }
    }
}

impl Settings {
    pub fn parse(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
}

/// Per-tab entries are keyed by surface id, which lives as long as the tab; groups outlive tabs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Manual order of every tab, once the user has moved one; empty keeps Haunt's order.
    pub order: Vec<String>,
    pub pinned: Vec<String>,
    pub groups: Vec<Group>,
    /// surface -> group id
    pub member: BTreeMap<String, String>,
    /// surface -> colour key from `COLORS`
    pub colors: BTreeMap<String, String>,
}

/// Where a row shows, as the core would place it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bucket {
    NeedsYou,
    Pinned,
    Section(String),
    State(StateGroup),
    Flat,
}

fn color(key: &str) -> Option<TabColor> {
    COLORS.iter().find(|(k, _, _)| *k == key).map(|(_, _, c)| *c)
}

/// A group name the core will show: 1 to 32 visible characters, not a core group's.
pub fn valid_name(name: &str) -> Option<String> {
    let n = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let shown = n.to_uppercase();
    let ok = !n.is_empty() && n.chars().count() <= MAX_NAME && !CORE_GROUPS.contains(&shown.as_str()) && !n.chars().any(char::is_control);
    ok.then_some(n)
}

impl State {
    pub fn parse(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Forgets tabs that are gone; their ids never come back.
    pub fn prune(&mut self, rows: &[RowInfo]) {
        let live = |s: &String| rows.iter().any(|r| &r.surface == s);
        self.order.retain(live);
        self.pinned.retain(live);
        self.member.retain(|s, _| live(s));
        self.colors.retain(|s, _| live(s));
        let groups = &self.groups;
        self.member.retain(|_, g| groups.iter().any(|x| &x.id == g));
    }

    fn rank(&self, rows: &[RowInfo], surface: &str) -> (usize, usize) {
        let core = rows.iter().position(|r| r.surface == surface).unwrap_or(usize::MAX);
        (self.order.iter().position(|s| s == surface).unwrap_or(usize::MAX), core)
    }

    fn sorted<'a>(&self, rows: &[RowInfo], mut v: Vec<&'a str>) -> Vec<&'a str> {
        v.sort_by_key(|s| self.rank(rows, s));
        v
    }

    pub fn bucket(&self, s: &Settings, row: &RowInfo) -> Bucket {
        if row.group == StateGroup::NeedsYou {
            return Bucket::NeedsYou;
        }
        if s.pins && self.pinned.contains(&row.surface) {
            return Bucket::Pinned;
        }
        if s.custom_groups && let Some(g) = self.member.get(&row.surface) {
            return Bucket::Section(g.clone());
        }
        if s.flat { Bucket::Flat } else { Bucket::State(row.group) }
    }

    /// The surfaces in `bucket`, in the order they show.
    pub fn in_bucket<'a>(&self, s: &Settings, rows: &'a [RowInfo], bucket: &Bucket) -> Vec<&'a str> {
        let v = rows.iter().filter(|r| &self.bucket(s, r) == bucket).map(|r| r.surface.as_str()).collect();
        if s.manual_order { self.sorted(rows, v) } else { v }
    }

    pub fn arrangement(&self, s: &Settings, rows: &[RowInfo]) -> Arrangement {
        let ids = |v: Vec<&str>| v.into_iter().map(str::to_owned).collect::<Vec<_>>();
        let pinned = if s.pins { ids(self.sorted(rows, self.pinned.iter().map(String::as_str).collect())) } else { vec![] };
        let sections = if s.custom_groups {
            self.groups
                .iter()
                .map(|g| Section {
                    id: g.id.clone(),
                    title: g.name.clone(),
                    surfaces: ids(self.sorted(rows, self.member.iter().filter(|(_, m)| **m == g.id).map(|(k, _)| k.as_str()).collect())),
                })
                .collect()
        } else {
            vec![]
        };
        Arrangement {
            pinned,
            sections,
            placement: if s.groups_placement == "after" { Placement::AfterState } else { Placement::BeforeState },
            order: if s.manual_order { self.order.clone() } else { vec![] },
            flat: s.flat,
            colors: if s.colors { self.colors.iter().filter_map(|(k, c)| Some((k.clone(), color(c)?))).collect() } else { vec![] },
            color_style: if s.color_style == "tint" { ColorStyle::Tint } else { ColorStyle::Dot },
        }
    }

    /// Writes the current display order into `order`, so a move has something to move within.
    fn materialize(&mut self, rows: &[RowInfo]) {
        let mut all: Vec<String> = rows.iter().map(|r| r.surface.clone()).collect();
        all.sort_by_key(|x| self.rank(rows, x));
        self.order = all;
    }

    fn place(&mut self, surface: &str, before: Option<&str>, after: Option<&str>) {
        self.order.retain(|x| x != surface);
        let at = match (before, after) {
            (Some(b), _) => self.order.iter().position(|x| x == b),
            (None, Some(a)) => self.order.iter().position(|x| x == a).map(|i| i + 1),
            (None, None) => None,
        };
        self.order.insert(at.unwrap_or(self.order.len()), surface.to_owned());
    }

    /// One step up (`-1`) or down (`1`) within the row's own group; false if it can't move.
    pub fn move_by(&mut self, s: &Settings, rows: &[RowInfo], surface: &str, delta: i32) -> bool {
        let Some(row) = rows.iter().find(|r| r.surface == surface) else { return false };
        let bucket = self.bucket(s, row);
        if !s.manual_order || bucket == Bucket::NeedsYou {
            return false;
        }
        self.materialize(rows);
        let list = self.in_bucket(s, rows, &bucket);
        let Some(i) = list.iter().position(|x| *x == surface) else { return false };
        let j = i as i64 + i64::from(delta);
        if j < 0 || j as usize >= list.len() {
            return false;
        }
        let other = list[j as usize].to_owned();
        if delta < 0 { self.place(surface, Some(&other), None) } else { self.place(surface, None, Some(&other)) }
        true
    }

    /// A drop into snapshot group `group` above `before`; false if nothing changed.
    pub fn move_to(&mut self, s: &Settings, rows: &[RowInfo], surface: &str, group: &str, before: Option<&str>) -> bool {
        if !rows.iter().any(|r| r.surface == surface) || group == "needs-you" {
            return false;
        }
        let before_state = self.clone();
        match group {
            "pinned" if s.pins => {
                self.member.remove(surface);
                if !self.pinned.iter().any(|x| x == surface) {
                    self.pinned.push(surface.to_owned());
                }
            }
            "running" | "idle" | "tabs" => {
                self.pinned.retain(|x| x != surface);
                self.member.remove(surface);
            }
            g => match g.strip_prefix("section:") {
                Some(id) if s.custom_groups && self.groups.iter().any(|x| x.id == id) => {
                    self.pinned.retain(|x| x != surface);
                    self.member.insert(surface.to_owned(), id.to_owned());
                }
                _ => return false,
            },
        }
        if s.manual_order {
            self.materialize(rows);
            let row = rows.iter().find(|r| r.surface == surface).expect("checked above");
            let last = self.in_bucket(s, rows, &self.bucket(s, row)).into_iter().rfind(|x| *x != surface).map(str::to_owned);
            match before.filter(|b| *b != surface && rows.iter().any(|r| r.surface == *b)) {
                Some(b) => self.place(surface, Some(b), None),
                None => self.place(surface, None, last.as_deref()),
            }
        }
        *self != before_state
    }

    pub fn toggle_pin(&mut self, surface: &str) {
        if self.pinned.iter().any(|x| x == surface) {
            self.pinned.retain(|x| x != surface);
        } else {
            self.member.remove(surface);
            self.pinned.push(surface.to_owned());
        }
    }

    /// The new group's id, or None if the name is not valid or there are 32 groups already.
    pub fn new_group(&mut self, name: &str) -> Option<String> {
        let name = valid_name(name)?;
        if self.groups.len() >= MAX_GROUPS {
            return None;
        }
        if let Some(g) = self.groups.iter().find(|g| g.name.to_uppercase() == name.to_uppercase()) {
            return Some(g.id.clone());
        }
        let slug: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
        let slug = slug.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
        let base = if slug.is_empty() { "group".to_owned() } else { slug.chars().take(48).collect() };
        let mut id = base.clone();
        let mut n = 2;
        while self.groups.iter().any(|g| g.id == id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        self.groups.push(Group { id: id.clone(), name });
        Some(id)
    }

    pub fn set_group(&mut self, surface: &str, group: Option<&str>) {
        match group.filter(|g| self.groups.iter().any(|x| x.id == *g)) {
            Some(g) => {
                self.pinned.retain(|x| x != surface);
                self.member.insert(surface.to_owned(), g.to_owned());
            }
            None => {
                self.member.remove(surface);
            }
        }
    }

    pub fn delete_group(&mut self, id: &str) {
        self.groups.retain(|g| g.id != id);
        self.member.retain(|_, g| g != id);
    }

    pub fn set_color(&mut self, surface: &str, key: Option<&str>) {
        match key.filter(|k| color(k).is_some()) {
            Some(k) => {
                self.colors.insert(surface.to_owned(), k.to_owned());
            }
            None => {
                self.colors.remove(surface);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, group: StateGroup) -> RowInfo {
        RowInfo { surface: id.into(), name: id.into(), kind: "claude".into(), group, selected: false }
    }

    /// Haunt's order: n (needs you), r1 r2 (running), i1 i2 (idle).
    fn rows() -> Vec<RowInfo> {
        use StateGroup::*;
        vec![row("n", NeedsYou), row("r1", Running), row("r2", Running), row("i1", Idle), row("i2", Idle)]
    }

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn nothing_changes_until_the_user_moves_a_tab() {
        let a = State::default().arrangement(&Settings::default(), &rows());
        assert!(a.pinned.is_empty() && a.sections.is_empty() && a.order.is_empty() && a.colors.is_empty() && !a.flat);
        assert_eq!(a.color_style, ColorStyle::Dot);
    }

    #[test]
    fn moving_stays_within_the_state_group() {
        let (s, rows) = (Settings::default(), rows());
        let mut st = State::default();
        assert!(!st.move_by(&s, &rows, "r1", -1), "already first in RUNNING");
        assert!(st.move_by(&s, &rows, "r1", 1));
        assert_eq!(st.in_bucket(&s, &rows, &Bucket::State(StateGroup::Running)), ["r2", "r1"]);
        assert!(!st.move_by(&s, &rows, "r1", 1), "already last in RUNNING; IDLE is another group");
        assert!(!st.move_by(&s, &rows, "n", 1), "a row needing you never moves");
    }

    #[test]
    fn order_sticks_when_a_tab_changes_state_group_and_back() {
        let s = Settings::default();
        let mut st = State::default();
        let mut rows = rows();
        assert!(st.move_by(&s, &rows, "r1", 1));
        rows[1].group = StateGroup::Idle;
        assert_eq!(st.in_bucket(&s, &rows, &Bucket::State(StateGroup::Idle)), ["r1", "i1", "i2"]);
        rows[1].group = StateGroup::Running;
        assert_eq!(st.in_bucket(&s, &rows, &Bucket::State(StateGroup::Running)), ["r2", "r1"]);
    }

    #[test]
    fn flat_mode_is_one_manual_list() {
        let s = Settings { flat: true, ..Settings::default() };
        let mut st = State::default();
        let rows = rows();
        assert!(st.move_by(&s, &rows, "i1", -1));
        assert_eq!(st.in_bucket(&s, &rows, &Bucket::Flat), ["r1", "i1", "r2", "i2"]);
        assert!(st.arrangement(&s, &rows).flat);
    }

    #[test]
    fn drops_pin_group_and_unpin() {
        let s = Settings::default();
        let rows = rows();
        let mut st = State::default();
        let work = st.new_group("Work").unwrap();
        assert!(st.move_to(&s, &rows, "i2", "pinned", None));
        assert!(st.move_to(&s, &rows, "r2", &format!("section:{work}"), None));
        assert!(st.move_to(&s, &rows, "i1", &format!("section:{work}"), Some("r2")));
        let a = st.arrangement(&s, &rows);
        assert_eq!(a.pinned, ids(&["i2"]));
        assert_eq!((a.sections[0].title.as_str(), a.sections[0].surfaces.clone()), ("Work", ids(&["i1", "r2"])));
        assert!(st.move_to(&s, &rows, "i2", "idle", None));
        assert!(st.arrangement(&s, &rows).pinned.is_empty());
        assert!(!st.move_to(&s, &rows, "r1", "needs-you", None));
        assert!(!st.move_to(&s, &rows, "r1", "section:nope", None));
    }

    #[test]
    fn groups_need_real_names_and_ids_stay_unique() {
        let mut st = State::default();
        for bad in ["", "   ", "needs you", "Pinned", "tabs", &"x".repeat(33)] {
            assert_eq!(st.new_group(bad), None, "{bad:?}");
        }
        assert_eq!(st.new_group("Side  projects").as_deref(), Some("side-projects"));
        assert_eq!(st.new_group("side projects").as_deref(), Some("side-projects"), "same name, same group");
        assert_eq!(st.new_group("Side/projects").as_deref(), Some("side-projects-2"));
        assert_eq!(st.new_group("☕").as_deref(), Some("group"));
        assert_eq!(st.groups[0].name, "Side projects");
    }

    #[test]
    fn deleting_a_group_frees_its_tabs() {
        let mut st = State::default();
        let g = st.new_group("Work").unwrap();
        st.set_group("r1", Some(&g));
        st.delete_group(&g);
        assert!(st.member.is_empty() && st.groups.is_empty());
    }

    #[test]
    fn pinning_and_grouping_are_exclusive() {
        let mut st = State::default();
        let g = st.new_group("Work").unwrap();
        st.set_group("r1", Some(&g));
        st.toggle_pin("r1");
        assert!(st.member.is_empty() && st.pinned == ["r1"]);
        st.set_group("r1", Some(&g));
        assert!(st.pinned.is_empty());
        st.toggle_pin("r1");
        st.toggle_pin("r1");
        assert!(st.pinned.is_empty());
    }

    #[test]
    fn colours_only_when_picked_and_enabled() {
        let mut st = State::default();
        st.set_color("r1", Some("blue"));
        st.set_color("r2", Some("magenta"));
        let rows = rows();
        assert_eq!(st.arrangement(&Settings::default(), &rows).colors, [("r1".to_owned(), TabColor::Blue)]);
        let tint = Settings { color_style: "tint".into(), ..Settings::default() };
        assert_eq!(st.arrangement(&tint, &rows).color_style, ColorStyle::Tint);
        let off = Settings { colors: false, ..Settings::default() };
        assert!(st.arrangement(&off, &rows).colors.is_empty());
        st.set_color("r1", None);
        assert!(st.colors.is_empty());
    }

    #[test]
    fn disabled_features_send_nothing() {
        let mut st = State::default();
        let g = st.new_group("Work").unwrap();
        st.set_group("r1", Some(&g));
        st.toggle_pin("r2");
        st.order = ids(&["r2", "r1"]);
        let off = Settings { pins: false, custom_groups: false, manual_order: false, ..Settings::default() };
        let a = st.arrangement(&off, &rows());
        assert!(a.pinned.is_empty() && a.sections.is_empty() && a.order.is_empty());
        assert_eq!(Settings::parse(r#"{"groups_placement":"after"}"#).groups_placement, "after");
        let after = Settings { groups_placement: "after".into(), ..Settings::default() };
        assert_eq!(st.arrangement(&after, &rows()).placement, Placement::AfterState);
    }

    #[test]
    fn closed_tabs_are_forgotten_and_state_round_trips() {
        let mut st = State::default();
        let g = st.new_group("Work").unwrap();
        st.set_group("gone", Some(&g));
        st.set_color("gone", Some("red"));
        st.toggle_pin("r1");
        st.order = ids(&["gone", "r1"]);
        st.prune(&rows());
        assert_eq!((st.order.clone(), st.pinned.clone()), (ids(&["r1"]), ids(&["r1"])));
        assert!(st.member.is_empty() && st.colors.is_empty() && st.groups.len() == 1, "groups outlive their tabs");
        assert_eq!(State::parse(&st.to_json()), st);
        assert_eq!(State::parse("not json"), State::default());
    }
}
