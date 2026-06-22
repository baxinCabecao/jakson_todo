use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<String>, // YYYY-MM-DD
    pub priority: String, // "high", "medium", "low"
    pub status: String, // "todo", "in_progress", "completed"
    pub created_at: String,
    pub subtasks: Option<Vec<Subtask>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subtask {
    pub id: Option<i64>,
    #[serde(default)]
    pub task_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub priority: String,
    pub completed: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub onedrive_client_id: Option<String>,
    pub onedrive_client_secret: Option<String>,
    pub onedrive_refresh_token: Option<String>,
    pub onedrive_enabled: bool,
    pub gdrive_client_id: Option<String>,
    pub gdrive_client_secret: Option<String>,
    pub gdrive_refresh_token: Option<String>,
    pub gdrive_enabled: bool,
    pub backup_frequency_mins: i64,
    pub last_backup_time: Option<String>,
}
