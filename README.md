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

## Develop

```bash
pnpm install
pnpm tauri dev      # needs the Rust toolchain (rustup) + AeroSpace for jump
```

For jumps to work: `aerospace enable on` (the app shows a hint + Enable button when the
server is off).

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
