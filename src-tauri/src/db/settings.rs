use rusqlite::{params, Result};
use crate::db::connection::DbConnection;
use crate::db::types::AppSettings;

impl DbConnection {
    /// Get settings from the database
    pub fn get_settings(&self) -> Result<AppSettings> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let val: String = row.get(1)?;
            Ok((key, val))
        })?;

        let mut onedrive_client_id = None;
        let mut onedrive_client_secret = None;
        let mut onedrive_refresh_token = None;
        let mut onedrive_enabled = false;
        let mut gdrive_client_id = None;
        let mut gdrive_client_secret = None;
        let mut gdrive_refresh_token = None;
        let mut gdrive_enabled = false;
        let mut backup_frequency_mins = 60;
        let mut last_backup_time = None;
        let mut last_safety_backup_time = None;

        for row in rows {
            let (key, val) = row?;
            match key.as_str() {
                "onedrive_client_id" => onedrive_client_id = Some(val),
                "onedrive_client_secret" => onedrive_client_secret = Some(val),
                "onedrive_refresh_token" => onedrive_refresh_token = Some(val),
                "onedrive_enabled" => onedrive_enabled = val == "1",
                "gdrive_client_id" => gdrive_client_id = Some(val),
                "gdrive_client_secret" => gdrive_client_secret = Some(val),
                "gdrive_refresh_token" => gdrive_refresh_token = Some(val),
                "gdrive_enabled" => gdrive_enabled = val == "1",
                "backup_frequency_mins" => {
                    if let Ok(num) = val.parse::<i64>() {
                        backup_frequency_mins = num;
                    }
                }
                "last_backup_time" => last_backup_time = Some(val),
                "last_safety_backup_time" => last_safety_backup_time = Some(val),
                _ => {}
            }
        }

        // Clean up Optional strings that are empty
        let clean_opt = |s: Option<String>| {
            match s {
                Some(ref v) if v.trim().is_empty() => None,
                other => other,
            }
        };

        Ok(AppSettings {
            onedrive_client_id: clean_opt(onedrive_client_id),
            onedrive_client_secret: clean_opt(onedrive_client_secret),
            onedrive_refresh_token: clean_opt(onedrive_refresh_token),
            onedrive_enabled,
            gdrive_client_id: clean_opt(gdrive_client_id),
            gdrive_client_secret: clean_opt(gdrive_client_secret),
            gdrive_refresh_token: clean_opt(gdrive_refresh_token),
            gdrive_enabled,
            backup_frequency_mins,
            last_backup_time: clean_opt(last_backup_time),
            last_safety_backup_time: clean_opt(last_safety_backup_time),
        })
    }

    /// Save a setting key-value pair
    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.get_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get a specific setting value by key
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let val: String = row.get(0)?;
            if val.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(val))
            }
        } else {
            Ok(None)
        }
    }

    /// Save all settings
    pub fn save_settings(&self, settings: AppSettings) -> Result<()> {
        let conn = self.get_conn()?;
        
        let fields = [
            ("onedrive_client_id", settings.onedrive_client_id.unwrap_or_default()),
            ("onedrive_client_secret", settings.onedrive_client_secret.unwrap_or_default()),
            ("onedrive_refresh_token", settings.onedrive_refresh_token.unwrap_or_default()),
            ("onedrive_enabled", if settings.onedrive_enabled { "1".to_string() } else { "0".to_string() }),
            ("gdrive_client_id", settings.gdrive_client_id.unwrap_or_default()),
            ("gdrive_client_secret", settings.gdrive_client_secret.unwrap_or_default()),
            ("gdrive_refresh_token", settings.gdrive_refresh_token.unwrap_or_default()),
            ("gdrive_enabled", if settings.gdrive_enabled { "1".to_string() } else { "0".to_string() }),
            ("backup_frequency_mins", settings.backup_frequency_mins.to_string()),
            ("last_backup_time", settings.last_backup_time.unwrap_or_default()),
            ("last_safety_backup_time", settings.last_safety_backup_time.unwrap_or_default()),
        ];

        for &(key, ref val) in &fields {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, val],
            )?;
        }

        Ok(())
    }
}
