use crate::backup::oauth::{get_effective_gdrive_client_id, get_effective_gdrive_client_secret};
use crate::backup::types::{GDriveListResponse, RemoteBackupInfo, TokenResponse};
use crate::db::AppSettings;

pub const SYNC_PAYLOAD_FILENAME: &str = "jakson_todo_sync_v2.json.gz";
pub const SAFETY_BACKUP_FILENAME: &str = "jakson_todo_safety_24h.json.gz";
pub const BACKUP_FILENAME: &str = "jakson_todo_sync_v2.json.gz";
pub const LEGACY_BACKUP_FILENAME: &str = "jakson_todo_backup.db";
const APPDATA_FOLDER: &str = "appDataFolder";

/// Refresh Google access token (supports PKCE with optional client_secret)
pub async fn refresh_gdrive_access_token(
    client_id: &str,
    refresh_token: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let mut params = vec![
        ("client_id", client_id),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    if let Some(sec) = get_effective_gdrive_client_secret() {
        params.push(("client_secret", sec));
    }

    let res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Falha ao renovar token do Google: {}", e))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(format!("Erro na API de renovação de token Google: {}", err_text));
    }

    let token_resp: TokenResponse = res
        .json()
        .await
        .map_err(|e| format!("Falha ao processar resposta de renovação: {}", e))?;

    Ok(token_resp.access_token)
}

/// Upload a file to Google Drive Application Data folder (appDataFolder)
pub async fn upload_file_to_gdrive(
    access_token: &str,
    filename: &str,
    description: &str,
    db_bytes: Vec<u8>,
) -> Result<(), String> {
    let client = reqwest::Client::new();

    // 1. Search if file already exists in appDataFolder
    let search_url = format!(
        "https://www.googleapis.com/drive/v3/files?spaces={}&q=name='{}' and trashed=false&fields=files(id,name)",
        APPDATA_FOLDER, filename
    );
    let search_res = client
        .get(&search_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Falha ao consultar pasta do aplicativo no Google Drive: {}", e))?;

    let list: GDriveListResponse = search_res
        .json()
        .await
        .map_err(|e| format!("Falha ao ler listagem de arquivos do Google Drive: {}", e))?;

    if let Some(existing_file) = list.files.first() {
        // Update existing file in appDataFolder
        let update_url = format!(
            "https://www.googleapis.com/upload/drive/v3/files/{}?uploadType=media",
            existing_file.id
        );
        let patch_res = client
            .patch(&update_url)
            .bearer_auth(access_token)
            .header("Content-Type", "application/octet-stream")
            .body(db_bytes)
            .send()
            .await
            .map_err(|e| format!("Falha ao atualizar arquivo no Google Drive: {}", e))?;

        if !patch_res.status().is_success() {
            let err_text = patch_res.text().await.unwrap_or_default();
            return Err(format!("Atualização no Google Drive falhou: {}", err_text));
        }
    } else {
        // Create new file inside appDataFolder (multipart)
        let metadata = serde_json::json!({
            "name": filename,
            "description": description,
            "parents": [APPDATA_FOLDER]
        });

        let form = reqwest::multipart::Form::new()
            .part(
                "metadata",
                reqwest::multipart::Part::text(metadata.to_string())
                    .mime_str("application/json; charset=UTF-8")
                    .unwrap(),
            )
            .part(
                "file",
                reqwest::multipart::Part::bytes(db_bytes)
                    .mime_str("application/octet-stream")
                    .unwrap(),
            );

        let upload_res = client
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
            .bearer_auth(access_token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Falha ao enviar arquivo para o Google Drive: {}", e))?;

        if !upload_res.status().is_success() {
            let err_text = upload_res.text().await.unwrap_or_default();
            return Err(format!("Criação do arquivo no Google Drive falhou: {}", err_text));
        }
    }

    Ok(())
}

#[allow(dead_code)]
/// Upload primary backup file to Google Drive
pub async fn upload_to_gdrive(
    access_token: &str,
    db_bytes: Vec<u8>,
) -> Result<(), String> {
    upload_file_to_gdrive(access_token, BACKUP_FILENAME, "Backup de dados do Jakson ToDo", db_bytes).await
}

#[allow(dead_code)]
/// Upload 24h safety backup file to Google Drive
pub async fn upload_safety_to_gdrive(
    access_token: &str,
    db_bytes: Vec<u8>,
) -> Result<(), String> {
    upload_file_to_gdrive(access_token, SAFETY_BACKUP_FILENAME, "Cópia de segurança 24h do Jakson ToDo", db_bytes).await
}

/// Download a file from Google Drive appDataFolder
pub async fn download_file_from_gdrive(
    access_token: &str,
    filename: &str,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();
    let search_url = format!(
        "https://www.googleapis.com/drive/v3/files?spaces={}&q=name='{}' and trashed=false&fields=files(id)",
        APPDATA_FOLDER, filename
    );
    let search_res = client
        .get(&search_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Falha ao buscar '{}' no Google Drive: {}", filename, e))?;

    let list: GDriveListResponse = search_res
        .json()
        .await
        .map_err(|e| format!("Falha ao processar lista de arquivos: {}", e))?;

    if let Some(file) = list.files.first() {
        let download_url = format!(
            "https://www.googleapis.com/drive/v3/files/{}?alt=media",
            file.id
        );
        let download_res = client
            .get(&download_url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| format!("Falha ao baixar '{}' do Google Drive: {}", filename, e))?;

        if !download_res.status().is_success() {
            let err_text = download_res.text().await.unwrap_or_default();
            return Err(format!("Erro no download do Google Drive: {}", err_text));
        }

        let bytes = download_res
            .bytes()
            .await
            .map_err(|e| format!("Falha ao ler dados baixados: {}", e))?
            .to_vec();

        Ok(bytes)
    } else {
        Err(format!("Arquivo '{}' não encontrado no Google Drive.", filename))
    }
}

/// Query Google Drive appDataFolder for file modified time
pub async fn get_gdrive_file_info(
    settings: &AppSettings,
    filename: &str,
) -> RemoteBackupInfo {
    let mut info = RemoteBackupInfo {
        provider: "GDrive".to_string(),
        exists: false,
        last_modified: None,
    };

    if !settings.gdrive_enabled {
        return info;
    }

    let refresh_token = match &settings.gdrive_refresh_token {
        Some(token) if !token.trim().is_empty() => token,
        _ => return info,
    };

    let client_id = get_effective_gdrive_client_id();

    let access_token = match refresh_gdrive_access_token(&client_id, refresh_token).await {
        Ok(token) => token,
        Err(_) => return info,
    };

    let client = reqwest::Client::new();
    let search_url = format!(
        "https://www.googleapis.com/drive/v3/files?spaces={}&q=name='{}' and trashed=false&fields=files(id,modifiedTime)",
        APPDATA_FOLDER, filename
    );

    if let Ok(res) = client.get(&search_url).bearer_auth(&access_token).send().await {
        if res.status().is_success() {
            if let Ok(list) = res.json::<GDriveListResponse>().await {
                if let Some(file) = list.files.first() {
                    info.exists = true;
                    info.last_modified = file.modified_time.clone();
                }
            }
        }
    }

    info
}

/// Query Google Drive appDataFolder for main backup file modified time
pub async fn get_gdrive_backup_info(
    settings: &AppSettings,
) -> RemoteBackupInfo {
    get_gdrive_file_info(settings, BACKUP_FILENAME).await
}

#[allow(dead_code)]
/// Query Google Drive appDataFolder for 24h safety backup file modified time
pub async fn get_gdrive_safety_backup_info(
    settings: &AppSettings,
) -> RemoteBackupInfo {
    get_gdrive_file_info(settings, SAFETY_BACKUP_FILENAME).await
}
