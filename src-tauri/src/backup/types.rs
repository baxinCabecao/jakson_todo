use crate::db::{Note, Task};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncPayload {
    pub version: u32,
    pub schema_version: u32,
    pub client_device_id: String,
    pub synced_at: String,
    pub tasks: Vec<Task>,
    pub notes: Vec<Note>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupStatus {
    pub provider: String,
    pub enabled: bool,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupReport {
    pub onedrive: BackupStatus,
    pub gdrive: BackupStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteBackupInfo {
    pub provider: String,
    pub exists: bool,
    pub last_modified: Option<String>, // ISO string
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CloudBackupsCheck {
    pub onedrive: RemoteBackupInfo,
    pub gdrive: RemoteBackupInfo,
    pub local_last_modified: String,
    pub newer_backup_available: bool,
    pub recommended_provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncResult {
    pub action: String, // "synced", "restored", "skipped", "none"
    pub message: String,
    pub safety_snapshot_taken: bool,
    pub provider: Option<String>,
    #[serde(default)]
    pub tasks_pulled: usize,
    #[serde(default)]
    pub tasks_pushed: usize,
    #[serde(default)]
    pub notes_pulled: usize,
    #[serde(default)]
    pub notes_pushed: usize,
}

// Structs for parsing OAuth and API responses
#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize)]
pub struct GDriveFile {
    pub id: String,
    #[serde(rename = "modifiedTime")]
    pub modified_time: Option<String>,
}

#[derive(Deserialize)]
pub struct GDriveListResponse {
    pub files: Vec<GDriveFile>,
}

#[derive(Deserialize)]
pub struct OneDriveFileResponse {
    #[serde(rename = "lastModifiedDateTime")]
    pub last_modified_date_time: Option<String>,
}
