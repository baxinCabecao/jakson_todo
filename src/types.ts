export interface Subtask {
  id?: number;
  uuid?: string;
  task_id?: number;
  title: string;
  description?: string;
  due_date?: string;
  priority: string; // "high", "medium", "low"
  completed: boolean;
  created_at: string;
  updated_at?: string;
}

export interface Task {
  id?: number;
  uuid?: string;
  title: string;
  description?: string;
  due_date?: string; // YYYY-MM-DD
  priority: string; // "high", "medium", "low"
  status: string; // "todo", "in_progress", "completed"
  created_at: string;
  updated_at?: string;
  is_deleted?: boolean;
  deleted_at?: string;
  subtasks?: Subtask[];
}

export interface Note {
  id?: number;
  uuid?: string;
  title: string;
  content: string;
  is_pinned: boolean;
  created_at: string;
  updated_at: string;
  is_deleted?: boolean;
  deleted_at?: string;
}

export interface AppSettings {
  onedrive_client_id?: string;
  onedrive_refresh_token?: string;
  onedrive_enabled: boolean;
  gdrive_client_id?: string;
  gdrive_refresh_token?: string;
  gdrive_enabled: boolean;
  backup_frequency_mins: number;
  last_backup_time?: string;
  last_safety_backup_time?: string;
}

export interface SyncResult {
  action: "restored" | "synced" | "skipped" | "none";
  message: string;
  safety_snapshot_taken: boolean;
  provider?: string;
  tasks_pulled?: number;
  tasks_pushed?: number;
  notes_pulled?: number;
  notes_pushed?: number;
}

export interface RemoteBackupInfo {
  provider: string;
  exists: boolean;
  last_modified?: string;
}

export interface CloudBackupsCheck {
  onedrive: RemoteBackupInfo;
  gdrive: RemoteBackupInfo;
  local_last_modified: string;
  newer_backup_available: boolean;
  recommended_provider?: string;
}
