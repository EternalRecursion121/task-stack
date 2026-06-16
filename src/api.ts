import { invoke } from "@tauri-apps/api/core";
import type { AeroStatus, Bootstrap, JumpType, Task, TaskState } from "./types";

export const api = {
  bootstrap: () => invoke<Bootstrap>("bootstrap"),
  listTasks: () => invoke<Task[]>("list_tasks"),

  createTask: (
    title: string,
    project: string | null,
    jumpType: JumpType | null,
    jumpValue: string | null,
  ) => invoke<Task>("create_task", { title, project, jumpType, jumpValue }),

  setState: (id: string, state: TaskState) =>
    invoke<Task>("set_state", { id, state }),

  updateTitle: (id: string, title: string, project: string | null) =>
    invoke<Task>("update_title", { id, title, project }),

  setNotes: (id: string, notes: string | null) =>
    invoke<Task>("set_notes", { id, notes }),

  setJump: (id: string, jumpType: JumpType | null, jumpValue: string | null) =>
    invoke<Task>("set_jump", { id, jumpType, jumpValue }),

  deleteTask: (id: string) => invoke<void>("delete_task", { id }),
  reorder: (ids: string[]) => invoke<void>("reorder", { ids }),

  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  setCorner: (corner: string) => invoke<void>("set_corner", { corner }),
  setHotkey: (hotkey: string) => invoke<void>("set_hotkey", { hotkey }),
  setCaptureHotkey: (hotkey: string) => invoke<void>("set_capture_hotkey", { hotkey }),
  setCaptureWsHotkey: (hotkey: string) => invoke<void>("set_capture_ws_hotkey", { hotkey }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),

  aerospaceStatus: () => invoke<AeroStatus>("aerospace_status"),
  aerospaceListWorkspaces: () => invoke<string[]>("aerospace_list_workspaces"),
  aerospaceFocusedWorkspace: () => invoke<string>("aerospace_focused_workspace"),
  aerospaceVisibleScene: () => invoke<string[]>("aerospace_visible_scene"),
  aerospaceEnable: () => invoke<void>("aerospace_enable"),

  jump: (jumpType: JumpType, jumpValue: string) =>
    invoke<void>("jump", { jumpType, jumpValue }),

  hideWindow: () => invoke<void>("hide_window"),
  setSize: (width: number, height: number) => invoke<void>("set_size", { width, height }),
};
