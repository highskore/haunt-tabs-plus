# haunt-tabs-plus

Tab arranging for the Haunt sidebar, as a first-party plugin (`dev.haunt.tabs-plus`). Every feature is a setting and can be turned off.

- **Manual order**: drag rows in the sidebar, press ⌥⌘↑ / ⌥⌘↓, or use "Move tab up" / "Move tab down" in the ⌘K palette or the row menu. A tab moves within its group. The order is one list across RUNNING and IDLE, so a tab that finishes and starts again keeps its place. With `flat = true` it is one manual list called TABS instead.
- **Custom groups**: in the palette, type `group `, then a name, and choose "New group". A tab joins a group from the row menu (Group ▸) or the same palette mode. The palette mode also removes a tab from its group and deletes groups. `groups_placement` puts them before or after RUNNING and IDLE.
- **Pin**: pinned tabs sit in PINNED at the top, right under NEEDS YOU.
- **Colour**: pick red, orange, yellow, green, blue, purple or gray from the row menu (Colour ▸) or with `color ` in the palette. `color_style = "dot"` shows a small dot beside the name; `"tint"` shows a faint row tint. No tab has a colour until you pick one.

A tab that needs you always shows in NEEDS YOU, first. That holds even when it is pinned, grouped or dragged: Haunt's core keeps that group whatever a plugin asks for. The tab goes back to its place when it stops needing you.

## Settings

| key | default | |
|---|---|---|
| `manual_order` | `true` | move tabs by dragging, ⌥⌘↑/↓ and the palette |
| `flat` | `false` | one manual list (TABS) instead of ordering within RUNNING and IDLE |
| `custom_groups` | `true` | named groups |
| `groups_placement` | `"before"` | `"before"` or `"after"` RUNNING and IDLE |
| `pins` | `true` | the PINNED section |
| `colors` | `true` | tab colours |
| `color_style` | `"dot"` | `"dot"` or `"tint"` |

Palette commands and modes are registered when the plugin loads, so turning a feature on or off fully applies after a restart. The sidebar follows a settings change right away.

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

It needs Haunt with `haunt:plugin@0.3` (`api = "0.3"`). `wit/haunt-plugin.wit` is a copy of Haunt's, pinned at that version.

## Licence

MIT, see `LICENSE`.
