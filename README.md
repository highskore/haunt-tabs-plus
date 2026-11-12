<div align="center">

<img src="docs/icon.png" width="96" alt="">

# Tabs Plus

**Tab arranging for the Haunt sidebar.**

[![Haunt plugin](https://img.shields.io/badge/Haunt-plugin-D97757)](https://github.com/highskore/haunt-hackathon)
![Plugin API 0.5](https://img.shields.io/badge/plugin%20API-0.5-111)
![WASM component](https://img.shields.io/badge/WASM-component-654ff0?logo=webassembly&logoColor=white)
![MIT licence](https://img.shields.io/badge/licence-MIT-6b7280)

</div>

Tab arranging for the Haunt sidebar, as a first-party plugin (`dev.haunt.tabs-plus`). Every feature is a setting and can be turned off.

- **Manual order**: drag rows in the sidebar, press ⌥⌘↑ / ⌥⌘↓, or use "Move tab up" / "Move tab down" in the ⌘K palette or the row menu. A tab moves within its group. The order is one list across RUNNING and IDLE, so a tab that finishes and starts again keeps its place. With `flat = true` it is one manual list called TABS instead.
- **Custom groups**: in the palette, type `group `, then a name, and choose "New group". A tab joins a group from the row menu (Group ▸) or the same palette mode. The palette mode also removes a tab from its group and deletes groups. `groups_placement` puts them before or after RUNNING and IDLE.
- **Pin**: pinned tabs sit in PINNED at the top, right under NEEDS YOU.
- **Colour**: pick red, orange, yellow, green, blue, purple or gray from the row menu (Colour ▸) or with `color ` in the palette. `color_style = "dot"` shows a small dot beside the name; `"tint"` shows a faint row tint. No tab has a colour until you pick one.

A tab that needs you always shows in NEEDS YOU, first. That holds even when it is pinned, grouped or dragged: Haunt's core keeps that group whatever a plugin asks for. The tab goes back to its place when it stops needing you.

## Settings

Set them in `~/.haunt/config.toml` under the plugin's id, for example:

```toml
[plugins."dev.haunt.tabs-plus"]
color_style = "tint"
pins = false
```

A key that isn't listed here, or a value of another type (`pins = "no"`), is skipped and logged. This needs a Haunt with plugin settings (`HauntPlugins.setConfig`).

| key | default | |
|---|---|---|
| `manual_order` | `true` | move tabs by dragging, ⌥⌘↑/↓ and the palette |
| `flat` | `false` | one manual list (TABS) instead of ordering within RUNNING and IDLE |
| `custom_groups` | `true` | named groups |
| `groups_placement` | `"before"` | `"before"` or `"after"` RUNNING and IDLE |
| `pins` | `true` | the PINNED section |
| `colors` | `true` | tab colours |
| `color_style` | `"dot"` | `"dot"` or `"tint"` |

The sidebar and the palette commands (Pin, Move tab up and down) follow a settings change as soon as you save. The `group` and `color` palette modes are registered when the plugin loads, so turning `custom_groups` or `colors` off removes them only after a restart.

## State

Everything lives in the plugin's kv, under `state`:

- the manual order, pins, group membership and colours, keyed by the tab's surface id
- the group names

A surface id lasts as long as its tab and is new after a restart. So the per-tab entries start over with each launch, and closed tabs are forgotten; the groups themselves are kept.

## Capabilities

| capability | why |
|---|---|
| `sidebar` | `sidebar.rows` (tab names, kinds and state groups; no task or command text) and `sidebar.arrange` |

It sees no agent events, files or terminal text.

## Build, test, install

```sh
cargo test                                     # native unit tests
cargo build --release --target wasm32-wasip2   # or ./build.sh
haunt plugin install .
```

Or from a haunt.app built with it (`macos/make-app.sh --plugins tabs-plus`): `haunt setup --plugins tabs-plus`, or check it in the setup sheet or Settings. Haunt never installs it on its own.

It needs Haunt with `haunt:plugin@0.5` (`api = "0.5"`). `wit/haunt-plugin.wit` is a copy of Haunt's, pinned at that version.

## Licence

MIT, see `LICENSE`.
