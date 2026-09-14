use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: Option<i64>,
    #[serde(default)]
    pub uuid: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<String>, // YYYY-MM-DD
    pub priority: String, // "high", "medium", "low"
    pub status: String, // "todo", "in_progress", "completed"
    pub created_at: String,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub deleted_at: Option<String>,
    pub subtasks: Option<Vec<Subtask>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subtask {
    pub id: Option<i64>,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub task_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub priority: String,
    pub completed: bool,
    pub created_at: String,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub id: Option<i64>,
    #[serde(default)]
    pub uuid: Option<String>,
    pub title: String,
    pub content: String,
    pub is_pinned: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub deleted_at: Option<String>,
}

fn default_backup_frequency() -> i64 {
    60
}

fn default_sidebar_pinned() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    #[serde(default)]
    pub onedrive_client_id: Option<String>,
    #[serde(default)]
    pub onedrive_client_secret: Option<String>,
    #[serde(default)]
    pub onedrive_refresh_token: Option<String>,
    #[serde(default)]
    pub onedrive_enabled: bool,
    #[serde(default)]
    pub gdrive_client_id: Option<String>,
    #[serde(default)]
    pub gdrive_client_secret: Option<String>,
    #[serde(default)]
    pub gdrive_refresh_token: Option<String>,
    #[serde(default)]
    pub gdrive_enabled: bool,
    #[serde(default = "default_backup_frequency")]
    pub backup_frequency_mins: i64,
    #[serde(default)]
    pub last_backup_time: Option<String>,
    #[serde(default)]
    pub last_safety_backup_time: Option<String>,
    #[serde(default = "default_sidebar_pinned")]
    pub desktop_sidebar_pinned: bool,
}
