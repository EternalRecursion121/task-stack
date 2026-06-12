import { invoke } from "@tauri-apps/api/core";
import type { AeroStatus, Bootstrap, JumpType, Task, TaskState, WaitingKind } from "./types";

export const api = {
  bootstrap: () => invoke<Bootstrap>("bootstrap"),
  listTasks: () => invoke<Task[]>("list_tasks"),

  createTask: (
    title: string,
    project: string | null,
    jump_type: JumpType | null,
    jump_value: string | null,
  ) => invoke<Task>("create_task", { title, project, jump_type, jump_value }),

  setState: (id: string, state: TaskState, waiting_kind: WaitingKind | null) =>
    invoke<Task>("set_state", { id, state, waiting_kind }),

  updateTitle: (id: string, title: string, project: string | null) =>
    invoke<Task>("update_title", { id, title, project }),

  setNotes: (id: string, notes: string | null) =>
    invoke<Task>("set_notes", { id, notes }),

  setJump: (id: string, jump_type: JumpType | null, jump_value: string | null) =>
    invoke<Task>("set_jump", { id, jump_type, jump_value }),

  deleteTask: (id: string) => invoke<void>("delete_task", { id }),
  reorder: (ids: string[]) => invoke<void>("reorder", { ids }),

  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  setCorner: (corner: string) => invoke<void>("set_corner", { corner }),
  setHotkey: (hotkey: string) => invoke<void>("set_hotkey", { hotkey }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),

  aerospaceStatus: () => invoke<AeroStatus>("aerospace_status"),
  aerospaceListWorkspaces: () => invoke<string[]>("aerospace_list_workspaces"),
  aerospaceFocusedWorkspace: () => invoke<string>("aerospace_focused_workspace"),
  aerospaceEnable: () => invoke<void>("aerospace_enable"),

  jump: (jump_type: JumpType, jump_value: string) =>
    invoke<void>("jump", { jump_type, jump_value }),

  hideWindow: () => invoke<void>("hide_window"),
  setSize: (width: number, height: number) => invoke<void>("set_size", { width, height }),
};
