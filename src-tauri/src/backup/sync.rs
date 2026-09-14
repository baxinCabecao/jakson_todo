use crate::backup::compression::{compress_db, decompress_db_if_needed};
use crate::backup::gdrive::{
    download_file_from_gdrive, refresh_gdrive_access_token, upload_file_to_gdrive,
    SAFETY_BACKUP_FILENAME as GDRIVE_SAFETY_NAME, SYNC_PAYLOAD_FILENAME as GDRIVE_SYNC_NAME,
};
use crate::backup::manager::get_or_create_device_id;
use crate::backup::oauth::{get_effective_gdrive_client_id, get_effective_onedrive_client_id};
use crate::backup::onedrive::{
    download_file_from_onedrive, refresh_onedrive_access_token, upload_file_to_onedrive,
    SAFETY_BACKUP_FILENAME as ONEDRIVE_SAFETY_NAME, SYNC_PAYLOAD_FILENAME as ONEDRIVE_SYNC_NAME,
};
use crate::backup::types::{SyncPayload, SyncResult};
use crate::db::{DbConnection, ReconcileStats};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

static SYNC_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Execute intelligent two-way synchronization:
/// 1. Acquire global synchronization lock to prevent race conditions on the same client.
/// 2. Download active sync payload from cloud (differentiating non-existent file from API errors).
/// 3. Two-way reconciliation: merge remote changes with local SQLite records (UUID + Last-Write-Wins + Tombstones).
/// 4. If remote had newer records: update local DB and emit "cloud-synced" event to frontend.
/// 5. If local had newer records or first sync: serialize and upload unified state to cloud.
/// 6. 24h Safety Snapshot: if >= 24h since last snapshot, upload safety snapshot.
pub async fn run_auto_sync(
    app_handle: &AppHandle,
    db_path: PathBuf,
) -> Result<SyncResult, String> {
    let _sync_guard = SYNC_MUTEX.lock().await;

    if !db_path.exists() {
        return Err("Arquivo de banco de dados não encontrado.".to_string());
    }

    let db = DbConnection::new(db_path.parent().unwrap().to_path_buf());
    let settings = db.get_settings().map_err(|e| e.to_string())?;

    if !settings.onedrive_enabled && !settings.gdrive_enabled {
        return Ok(SyncResult {
            action: "none".to_string(),
            message: "Nenhum provedor de sincronização está ativado.".to_string(),
            safety_snapshot_taken: false,
            provider: None,
            tasks_pulled: 0,
            tasks_pushed: 0,
            notes_pulled: 0,
            notes_pushed: 0,
        });
    }

    // Refresh cloud access tokens for enabled providers
    let mut gdrive_token: Option<String> = None;
    if settings.gdrive_enabled {
        if let Some(ref ref_token) = settings.gdrive_refresh_token {
            let client_id = get_effective_gdrive_client_id();
            if let Ok(token) = refresh_gdrive_access_token(&client_id, ref_token).await {
                gdrive_token = Some(token);
            }
        }
    }

    let mut onedrive_token: Option<String> = None;
    if settings.onedrive_enabled {
        if let Some(ref ref_token) = settings.onedrive_refresh_token {
            let client_id = get_effective_onedrive_client_id();
            if !client_id.is_empty() {
                if let Ok(token) = refresh_onedrive_access_token(&client_id, ref_token).await {
                    onedrive_token = Some(token);
                }
            }
        }
    }

    if gdrive_token.is_none() && onedrive_token.is_none() {
        return Err("Nenhum provedor de nuvem conectado com token válido.".to_string());
    }

    // 1. Download remote sync payload
    let mut remote_payload: Option<SyncPayload> = None;
    let mut is_first_sync = false;

    // Try Google Drive first if available
    if let Some(ref token) = gdrive_token {
        match download_file_from_gdrive(token, GDRIVE_SYNC_NAME).await {
            Ok(Some(raw)) => {
                let decompressed = decompress_db_if_needed(&raw)
                    .map_err(|e| format!("Falha ao descomprimir payload do Google Drive: {}", e))?;
                let payload = serde_json::from_slice::<SyncPayload>(&decompressed)
                    .map_err(|e| format!("Falha ao processar payload do Google Drive: {}", e))?;
                remote_payload = Some(payload);
            }
            Ok(None) => {
                // File legitimately does not exist yet on GDrive
                is_first_sync = true;
            }
            Err(e) => {
                // Network or API failure: abort immediately to never overwrite remote data
                return Err(format!("Erro ao acessar Google Drive: {}", e));
            }
        }
    }

    // Check OneDrive if GDrive was not configured or genuinely had no file yet
    if remote_payload.is_none() && onedrive_token.is_some() && (gdrive_token.is_none() || is_first_sync) {
        if let Some(ref token) = onedrive_token {
            match download_file_from_onedrive(token, ONEDRIVE_SYNC_NAME).await {
                Ok(Some(raw)) => {
                    let decompressed = decompress_db_if_needed(&raw)
                        .map_err(|e| format!("Falha ao descomprimir payload do OneDrive: {}", e))?;
                    let payload = serde_json::from_slice::<SyncPayload>(&decompressed)
                        .map_err(|e| format!("Falha ao processar payload do OneDrive: {}", e))?;
                    remote_payload = Some(payload);
                    is_first_sync = false;
                }
                Ok(None) => {
                    is_first_sync = true;
                }
                Err(e) => {
                    return Err(format!("Erro ao acessar OneDrive: {}", e));
                }
            }
        }
    }

    // 2. Perform two-way reconciliation with local database
    let stats: ReconcileStats = if let Some(ref payload) = remote_payload {
        db.reconcile_with_remote(&payload.tasks, &payload.notes)
            .map_err(|e| format!("Erro ao reconciliar dados: {}", e))?
    } else if is_first_sync {
        // First sync: all local items will be pushed to the cloud
        let total_tasks = db.get_all_sync_tasks().map(|t| t.len()).unwrap_or(0);
        let total_notes = db.get_all_sync_notes().map(|n| n.len()).unwrap_or(0);
        ReconcileStats {
            tasks_pulled: 0,
            tasks_pushed: total_tasks,
            notes_pulled: 0,
            notes_pushed: total_notes,
        }
    } else {
        return Err("Falha ao determinar o estado da sincronização na nuvem.".to_string());
    };

    // 3. Notify frontend if any records were pulled/updated from the cloud
    if stats.has_pulled_changes() {
        let _ = app_handle.emit("cloud-synced", serde_json::json!({
            "tasks_pulled": stats.tasks_pulled,
            "notes_pulled": stats.notes_pulled,
        }));
        // Emit legacy event as well for backward compatibility
        let _ = app_handle.emit("cloud-restored", serde_json::json!({ "provider": "cloud" }));
    }

    // 4. Check if cloud payload needs updating
    let now = Utc::now();
    let now_str = now.to_rfc3339();

    let should_take_safety_snapshot = match &settings.last_safety_backup_time {
        Some(time_str) => match DateTime::parse_from_rfc3339(time_str) {
            Ok(dt) => now.signed_duration_since(dt.with_timezone(&Utc)).num_seconds() >= 24 * 3600,
            Err(_) => true,
        },
        None => true,
    };

    // Only upload to the cloud if we have local changes to push, if it's the very first sync, or if 24h snapshot is due.
    // Notice: if we only pulled remote changes and had no local changes, we do NOT re-upload (prevents ping-pong loops).
    let needs_upload = stats.has_pushed_changes() || is_first_sync || should_take_safety_snapshot;

    if needs_upload {
        let all_tasks = db.get_all_sync_tasks().map_err(|e| e.to_string())?;
        let all_notes = db.get_all_sync_notes().map_err(|e| e.to_string())?;

        let payload = SyncPayload {
            version: 2,
            schema_version: 1,
            client_device_id: get_or_create_device_id(&db),
            synced_at: now_str.clone(),
            tasks: all_tasks,
            notes: all_notes,
        };

        let json_bytes = serde_json::to_vec(&payload)
            .map_err(|e| format!("Falha ao serializar dados de sincronização: {}", e))?;
        let compressed_bytes = compress_db(&json_bytes)
            .map_err(|e| format!("Falha ao comprimir dados de sincronização: {}", e))?;

        // Upload to Google Drive if active
        if let Some(ref token) = gdrive_token {
            upload_file_to_gdrive(
                token,
                GDRIVE_SYNC_NAME,
                "Dados de Sincronização Jakson ToDo",
                compressed_bytes.clone(),
            ).await
            .map_err(|e| format!("Falha no envio para o Google Drive: {}", e))?;
        }

        // Upload to OneDrive if active
        if let Some(ref token) = onedrive_token {
            upload_file_to_onedrive(
                token,
                ONEDRIVE_SYNC_NAME,
                compressed_bytes.clone(),
            ).await
            .map_err(|e| format!("Falha no envio para o OneDrive: {}", e))?;
        }

        // 5. 24-Hour Safety Snapshot
        if should_take_safety_snapshot {
            if let Some(ref token) = gdrive_token {
                let _ = upload_file_to_gdrive(
                    token,
                    GDRIVE_SAFETY_NAME,
                    "Snapshot de Segurança 24h Jakson ToDo",
                    compressed_bytes.clone(),
                ).await;
            }

            if let Some(ref token) = onedrive_token {
                let _ = upload_file_to_onedrive(
                    token,
                    ONEDRIVE_SAFETY_NAME,
                    compressed_bytes.clone(),
                ).await;
            }

            // Save local safety snapshot
            let local_safety_path = db_path.parent().unwrap().join("todo_safety_24h.json.gz");
            let _ = fs::write(&local_safety_path, &compressed_bytes);
            let _ = db.save_setting("last_safety_backup_time", &now_str);
        }

        let _ = db.save_setting("last_backup_time", &now_str);
    }

    // Clean up soft-deleted tombstones older than 30 days locally
    let _ = db.purge_old_tombstones(30);

    let active_provider = if gdrive_token.is_some() {
        "Google Drive".to_string()
    } else {
        "OneDrive".to_string()
    };

    let summary_msg = if stats.tasks_pulled > 0 || stats.notes_pulled > 0 || stats.tasks_pushed > 0 || stats.notes_pushed > 0 {
        format!(
            "Sincronização concluída: {} itens atualizados, {} enviados.",
            stats.tasks_pulled + stats.notes_pulled,
            stats.tasks_pushed + stats.notes_pushed
        )
    } else {
        "Sincronização em dia (nenhuma alteração pendente).".to_string()
    };

    Ok(SyncResult {
        action: "synced".to_string(),
        message: summary_msg,
        safety_snapshot_taken: should_take_safety_snapshot,
        provider: Some(active_provider),
        tasks_pulled: stats.tasks_pulled,
        tasks_pushed: stats.tasks_pushed,
        notes_pulled: stats.notes_pulled,
        notes_pushed: stats.notes_pushed,
    })
}
