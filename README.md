# task-stack

An always-on, corner-pinned heads-up display for multiplexing parallel work — multiple
Claude Code instances, writing, research. Not a task manager and not a kanban board: it
answers two questions at a glance — *which thread needs me?* and *take me there.*

- **Grouped triage list**, not a board: collapsible **Active / Waiting on you / Running /
  Done** sections in one narrow glass panel.
- **Waiting splits two ways** — *waiting on you* (an agent is blocked, surfaced loud in
  amber) vs *running* (the machine is grinding, muted blue).
- **Click a task → jump to its [AeroSpace](https://github.com/nikitabobko/AeroSpace)
  workspace.** Bindings are by workspace name, so they survive restarts.
- Lives in the **tray** (no dock icon), summoned by a **global hotkey**, pinned to a
  configurable corner.

## Stack

- **Tauri v2** (Rust) — tiny footprint for an always-on widget.
- **React + Vite + Tailwind v4** frontend; native macOS `hudWindow` vibrancy for the glass.
- **SQLite** (WAL) at `~/.task-stack/state.db` — shared with the future CLI/hooks.
- Window focus is delegated to the `aerospace` CLI (no raw Accessibility code).

## Setup

**Prerequisites**

- [Rust toolchain](https://rustup.rs) (`rustup`) and Node + `pnpm`.
- [AeroSpace](https://github.com/nikitabobko/AeroSpace) for workspace jumps, with its server
  enabled: `aerospace enable on` (the app also shows an Enable button when the server is off).

**Run it**

```bash
pnpm install
pnpm tauri dev
```

**Let AeroSpace leave the panel alone (important).** AeroSpace is a tiling window manager, so
by default it will try to *tile* the always-on-top widget — which fights the app's own corner
positioning in a feedback loop and hangs/crashes it. Add a float rule to `~/.aerospace.toml`
so the panel is always floating, then `aerospace reload-config`:

```toml
[[on-window-detected]]
if.app-id = 'com.samuelratnam.taskstack'    # task-stack corner widget — never tile it
run = 'layout floating'
```

**Hotkey — Cmd+Space.** The default summon hotkey is **⌘Space**, which macOS assigns to
Spotlight. To free it: System Settings → Keyboard → Keyboard Shortcuts → Spotlight → uncheck
*Show Spotlight search* (or `defaults write com.apple.symbolichotkeys AppleSymbolicHotKeys
-dict-add 64 "{enabled = 0;}"` then log out/in). Pick any other accelerator in the app's
Settings if you'd rather keep Spotlight on ⌘Space — just avoid Option-based chords, which
AeroSpace owns.

> **Packaging note:** launched as a bundled `.app` (from Finder/Spotlight), the process does
> **not** inherit your shell `PATH`, so `aerospace` in `/opt/homebrew/bin` won't be found and
> jumps will report "not installed." This only affects packaged builds — `pnpm tauri dev`
> inherits your terminal's PATH. Resolve the absolute path before shipping.

## Usage

- **Summon / hide:** ⌘Space — appears in the corner of whatever monitor your cursor is on.
- **Add a task:** type in the field, `#tag` sets the project chip, Enter to add.
- **Set state:** hover a row → ◑ cycles *active → waiting on you → running*; ✓ completes
  (drops into the collapsed Done section, recoverable via ↺).
- **Bind a jump target:** hover → ◎ captures the current arrangement across **all** monitors
  (a *scene*); **⌥-click ◎** captures just the focused monitor's workspace. Click the row to
  jump back to it. Drag rows to reorder within a group.
- **Move the panel:** **⌘+arrow keys** snap it between corners while it's focused (⌘← / ⌘→
  pick the side, ⌘↑ / ⌘↓ pick top/bottom); or set the corner in Settings.

## Layout

```
src/                  React UI — App.tsx (groups, dnd, resize), components/{TaskRow,Settings}
src-tauri/src/
  lib.rs              window shell, tray, hotkey, autostart, IPC commands
  db.rs               SQLite schema + task/settings CRUD
  aerospace.rs        thin wrapper over the `aerospace` CLI (+ disabled-server detection)
```

## Roadmap (phase 2)

The status model and DB are designed so this can land without a rewrite:

- a `task-stack` **CLI** writing the same SQLite DB, and
- **Claude Code hooks** so VM instances auto-report status (permission prompt →
  *waiting on you*; Stop → done; long tool runs → *running*).

See the design spec in `~/.claude/plans/` for the full rationale.
