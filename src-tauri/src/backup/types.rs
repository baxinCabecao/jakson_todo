use serde::{Deserialize, Serialize};

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
