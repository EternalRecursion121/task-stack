export type TaskState = "active" | "waiting" | "done";
export type WaitingKind = "me" | "machine";
export type JumpType = "workspace" | "window" | "url" | "command";

export interface Task {
  id: string;
  title: string;
  state: TaskState;
  waiting_kind: WaitingKind | null;
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
  jump_mode: string;
  auto_collapse: boolean;
}

export interface Bootstrap extends Settings {
  tasks: Task[];
  aerospace: AeroStatus;
}
