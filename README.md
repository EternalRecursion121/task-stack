# task-stack

An always-on, corner-pinned heads-up display for multiplexing parallel work — multiple
Claude Code instances, writing, research. Not a task manager and not a kanban board: it
answers two questions at a glance — *which thread needs me?* and *take me there.*

- **One narrow glass panel**, not a board: a collapsible **Active** list with a tucked-away
  **Done** section. Drag to reorder; **defer** (↓) drops a thread to the bottom so whatever
  needs you next rises to the top.
- **Click a task → jump to its [AeroSpace](https://github.com/nikitabobko/AeroSpace)
  workspace.** Bind a single workspace *or* a whole multi-monitor **scene**; bindings are by
  workspace name, so they survive restarts.
- Lives in the **tray** (no dock icon), summoned by a **global hotkey**, pinned to a
  configurable corner of whichever monitor your cursor is on.

## Stack

- **Tauri v2** (Rust) — tiny footprint for an always-on widget.
- **React + Vite + Tailwind v4** frontend; native macOS `hudWindow` vibrancy for the glass.
- **SQLite** (WAL) at `~/.task-stack/state.db` — shared with the future CLI/hooks.
- Window focus is delegated to the `aerospace` CLI (no raw Accessibility code).

## Requirements

- **macOS.** The widget is a native `NSPanel`, and AeroSpace is macOS-only.
- [**Rust toolchain**](https://rustup.rs) (`rustup`) and **Xcode Command Line Tools**
  (`xcode-select --install`).
- **Node 18+** and [**pnpm**](https://pnpm.io) (`npm i -g pnpm`).
- [**AeroSpace**](https://github.com/nikitabobko/AeroSpace) for workspace jumps, with its
  server enabled: `aerospace enable on` (the app also shows an **Enable** button when the
  server is off). `aerospace` must be on your `PATH` or in a standard Homebrew location
  (`/opt/homebrew/bin` or `/usr/local/bin`) — the app looks there directly, so jumps work
  even from a bundled `.app` that doesn't inherit your shell `PATH`.

## Run / build

```bash
pnpm install
pnpm tauri dev      # hot-reloading dev build
pnpm tauri build    # packaged .app + .dmg under src-tauri/target/release/bundle/
```

## Hotkeys

| Action                          | Default | Notes                                                       |
| ------------------------------- | ------- | ----------------------------------------------------------- |
| Summon / hide the panel         | ⌘Space  | macOS uses this for Spotlight — see *Conflicts* below       |
| Capture scene (all monitors)    | ⌘⇧Space | creates a task bound to every monitor's visible workspace   |
| Capture focused workspace       | ⌘⌥Space | creates a task bound to the focused workspace only          |
| Snap the panel between corners  | ⌘← ⌘→ ⌘↑ ⌘↓ | only while the panel is focused                         |

All of these are configurable in **Settings** (clear a capture field to disable that hotkey).
A capture hotkey summons the panel and drops a new, workspace-bound task straight into rename
mode so you can name the thread without touching the mouse.

**Conflicts to know about** — task-stack can't override shortcuts another app grabs first:

- **⌘Space is Spotlight.** Free it (System Settings → Keyboard → Keyboard Shortcuts →
  Spotlight → uncheck *Show Spotlight search*) or choose a different summon key in Settings.
- **⌘⌥Space is "Show Finder search window"** by default on macOS. Rebind either that system
  shortcut or the capture-workspace hotkey so they don't collide.
- **⌘ + arrows** are commonly claimed by window managers / launchers (e.g. Raycast's window
  management). A system-level grab beats task-stack's handler, so if corner-snapping does
  nothing, clear that binding in the other app. Avoid Option-only chords — AeroSpace owns
  those.

## Usage

- **Add a task:** type in the field, `#tag` sets the project chip, Enter to add.
- **Triage:** hover a row → **↓** defers it to the bottom of the active queue; **✓** completes
  it (drops into the collapsed **Done** section, recoverable with **↺**). Drag rows to reorder.
- **Bind a jump target:** hover → **◎** captures the current arrangement across **all**
  monitors (a *scene*, shown on the row as **⧉ N**); **⌥-click ◎** captures just the focused
  monitor's workspace (shown as **▭**). Or use the capture hotkeys above to do it hands-free.
  Click the row to jump back to its target.
- **Jump mode (Settings → "Click jumps by"):** *focus* switches to the bound workspace in
  place; *pull here* summons its windows onto your current monitor instead.
- **Move the panel:** ⌘+arrow keys snap it between corners while it's focused (⌘← / ⌘→ pick the
  side, ⌘↑ / ⌘↓ pick top/bottom), or set the corner in Settings.

## Tiling window managers

The widget is a non-activating macOS `NSPanel`, so tiling WMs like AeroSpace ignore it
automatically — no config needed, and summoning it never steals focus from the app underneath.
If you ever do see it get tiled, add a float rule to `~/.aerospace.toml` and
`aerospace reload-config` (match by name — the dev build reports a null bundle id):

```toml
[[on-window-detected]]
if.app-name-regex-substring = 'task-stack'    # never tile the corner widget
run = 'layout floating'
```

## Layout

```
src/                  React UI — App.tsx (groups, dnd, resize, hotkey events),
                      components/{TaskRow,Settings}
src-tauri/src/
  lib.rs              window shell, tray, hotkeys, autostart, IPC commands
  db.rs               SQLite schema + task/settings CRUD
  aerospace.rs        thin wrapper over the `aerospace` CLI (+ disabled-server detection)
```

## Roadmap (phase 2)

The status model and DB are designed so this can land without a rewrite:

- a `task-stack` **CLI** writing the same SQLite DB, and
- **Claude Code hooks** so VM instances auto-report status (permission prompt → surfaced for
  you; Stop → done; long tool runs → in progress).

See the design spec in `~/.claude/plans/` for the full rationale.
