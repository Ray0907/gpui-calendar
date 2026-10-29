# gpui-calendar

A desktop calendar written in Rust with [GPUI](https://www.gpui.rs) through the `gpui-kit` component crate. Day, week, month (infinite scroll) and year views; drag to create, move and resize events at a 15-minute snap; an event popover; a mini month and a calendar list; light, dark or system appearance with preset colors. Everything is local: no server, no network.

## Requirements

- Rust 1.99.1, pinned by `rust-toolchain.toml` (rustup picks it up on its own).
- macOS is the primary target. The headless screenshots need GPUI's Metal renderer, so `cargo run --example shots` does nothing elsewhere.

## First build

`vendor/` is git-ignored. On a fresh clone run `build/vendor-gpui-component.sh` once before `cargo build`: it vendors gpui-component (kit 0.8.1) and applies `patches/*.patch`. Without it cargo fails at manifest load with a path-dependency error.

## Run

```sh
cargo run                      # open the calendar
cargo test                     # unit, headless UI and E2E tests
cargo test e2e                 # only the create/move/resize/recolor/relaunch test
cargo run --example shots      # light and dark screenshots into artifacts/
```

`shots` writes `day`, `week`, `month`, `year` and `appearance` as `<name>-light.png` and `<name>-dark.png`, plus `shell-*` and `popover-*`, and exits non-zero if any of the five views is missing or empty.

## Keyboard shortcuts

| Key | Action |
|---|---|
| `t` | go to today |
| `d` `w` `m` `y` | day, week, month, year view |
| `n` | new event |
| Left / Right arrows | previous / next period |
| Tab, arrows | one Tab stop per grid; arrows move inside it |
| Esc | close the popover, cancel a drag |

Shortcuts are off while typing in a field or while the popover is open.

## Data file

Events, calendar visibility and settings (week start, time format, appearance) live in one versioned JSON file:

- macOS: `~/Library/Application Support/gpui-calendar/calendar.json`
- Windows: `%APPDATA%\gpui-calendar\calendar.json`
- Linux: `$XDG_CONFIG_HOME/gpui-calendar/calendar.json` (or `~/.config/...`)

Saves are atomic. A file that is damaged or from a newer version is kept as `calendar.json.bak`, defaults are shown, and a notice says so. Tests and screenshots use a temp directory, never this path.

## Known gaps

- Not verified: how GPUI's renderer blends the 18% and 16% tints.
- Press feedback is a dim, not the scale the guide describes.
- No letter-spacing.
- Reduced motion is read from the `REDUCE_MOTION` environment variable only (GPUI does not yet read the OS setting), e.g. `REDUCE_MOTION=1 cargo run`.
- Not verified: screen reader and forced-colors behavior, right-to-left layout, windows narrower than 960 px.
- Week start and 12/24-hour time are stored in the file but have no control in the app yet.
