export type TaskState = "active" | "done";
export type JumpType = "workspace" | "scene" | "window" | "url" | "command";

export interface Task {
  id: string;
  title: string;
  state: TaskState;
  project: string | null;
  jump_type: JumpType | null;
  jump_value: string | null;
  notes: string | null;
  sort: number;
  created_at: number;
  updated_at: number;
  completed_at: number | null;
}

export interface AeroStatus {
  installed: boolean;
  server_enabled: boolean;
  message: string | null;
}

export interface Settings {
  corner: string;
  hotkey: string;
  capture_hotkey: string;
  capture_ws_hotkey: string;
  jump_mode: string;
  auto_collapse: boolean;
}

export interface Bootstrap extends Settings {
  tasks: Task[];
  aerospace: AeroStatus;
}
