import React, { useState, useEffect } from "react";
import { Cloud, Clock, RefreshCw, Database } from "lucide-react";
import { AppSettings } from "../types";
import "./Settings.css";

interface SettingsProps {
  settings: AppSettings;
  onSaveSettings: (settings: AppSettings) => void;
  onConnectProvider: (provider: "gdrive" | "onedrive", clientId: string, clientSecret: string) => void;
  onRestoreBackup: (provider: string) => void;
  isRestoring: boolean;
  isBackingUp: boolean;
  backupReport: any;
  onManualBackup: () => void;
}

export const Settings: React.FC<SettingsProps> = ({
  settings,
  onSaveSettings,
  onConnectProvider,
  onRestoreBackup,
  isRestoring,
  isBackingUp,
  backupReport,
  onManualBackup,
}) => {
  const [gdriveClientId, setGdriveClientId] = useState("");
  const [gdriveClientSecret, setGdriveClientSecret] = useState("");
  const [onedriveClientId, setOnedriveClientId] = useState("");
  const [onedriveClientSecret, setOnedriveClientSecret] = useState("");
  const [backupFrequencyMins, setBackupFrequencyMins] = useState(60);
  const [gdriveEnabled, setGdriveEnabled] = useState(false);
  const [onedriveEnabled, setOnedriveEnabled] = useState(false);

  // Sync state with settings prop
  useEffect(() => {
    setGdriveClientId(settings.gdrive_client_id || "");
    setGdriveClientSecret(settings.gdrive_client_secret || "");
    setOnedriveClientId(settings.onedrive_client_id || "");
    setOnedriveClientSecret(settings.onedrive_client_secret || "");
    setBackupFrequencyMins(settings.backup_frequency_mins);
    setGdriveEnabled(settings.gdrive_enabled);
    setOnedriveEnabled(settings.onedrive_enabled);
  }, [settings]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const updatedSettings: AppSettings = {
      ...settings,
      gdrive_client_id: gdriveClientId.trim() ? gdriveClientId : undefined,
      gdrive_client_secret: gdriveClientSecret.trim() ? gdriveClientSecret : undefined,
      onedrive_client_id: onedriveClientId.trim() ? onedriveClientId : undefined,
      onedrive_client_secret: onedriveClientSecret.trim() ? onedriveClientSecret : undefined,
      backup_frequency_mins: backupFrequencyMins,
      gdrive_enabled: gdriveEnabled,
      onedrive_enabled: onedriveEnabled,
    };
    onSaveSettings(updatedSettings);
  };

  return (
    <div className="tab-pane animate-fade-in">
      <header className="content-header">
        <div>
          <h1>Configurações da Aplicação</h1>
          <p>Gerencie seus provedores de backup na nuvem e preferências globais.</p>
        </div>
      </header>

      <form onSubmit={handleSubmit} className="settings-form">
        {/* Google Drive Configuration Card */}
        <section className="settings-section-card">
          <div className="section-header">
            <Cloud size={20} className="section-icon" />
            <h2>Integração com Google Drive</h2>
          </div>

          <div className="form-group-row">
            <div className="form-group-field">
              <label>Client ID do Google Drive</label>
              <input
                type="text"
                placeholder="Ex: 123456789-abc.apps.googleusercontent.com"
                value={gdriveClientId}
                onChange={(e) => setGdriveClientId(e.target.value)}
              />
            </div>
            <div className="form-group-field">
              <label>Client Secret do Google Drive</label>
              <input
                type="password"
                placeholder="••••••••••••••••"
                value={gdriveClientSecret}
                onChange={(e) => setGdriveClientSecret(e.target.value)}
              />
            </div>
          </div>

          <div className="provider-status-row">
            <div
              className={`status-badge ${
                settings.gdrive_enabled ? "connected" : "disconnected"
              }`}
            >
              {settings.gdrive_enabled ? "Habilitado e Conectado" : "Não Conectado"}
            </div>
            <button
              type="button"
              className="btn-secondary"
              onClick={() => onConnectProvider("gdrive", gdriveClientId, gdriveClientSecret)}
            >
              {settings.gdrive_enabled ? "Reautorizar GDrive" : "Conectar Google Drive"}
            </button>
            <label className="toggle-backup-label">
              <input
                type="checkbox"
                checked={gdriveEnabled}
                onChange={(e) => setGdriveEnabled(e.target.checked)}
              />
              <span>Ativar Google Drive Backup</span>
            </label>
          </div>
        </section>

        {/* OneDrive Configuration Card */}
        <section className="settings-section-card">
          <div className="section-header">
            <Cloud size={20} className="section-icon" />
            <h2>Integração com Microsoft OneDrive</h2>
          </div>

          <div className="form-group-row">
            <div className="form-group-field">
              <label>Client ID do OneDrive</label>
              <input
                type="text"
                placeholder="Ex: 5d1345a-cf2a-43d2"
                value={onedriveClientId}
                onChange={(e) => setOnedriveClientId(e.target.value)}
              />
            </div>
            <div className="form-group-field">
              <label>Client Secret do OneDrive (Opcional)</label>
              <input
                type="password"
                placeholder="••••••••••••••••"
                value={onedriveClientSecret}
                onChange={(e) => setOnedriveClientSecret(e.target.value)}
              />
            </div>
          </div>

          <div className="provider-status-row">
            <div
              className={`status-badge ${
                settings.onedrive_enabled ? "connected" : "disconnected"
              }`}
            >
              {settings.onedrive_enabled ? "Habilitado e Conectado" : "Não Conectado"}
            </div>
            <button
              type="button"
              className="btn-secondary"
              onClick={() => onConnectProvider("onedrive", onedriveClientId, onedriveClientSecret)}
            >
              {settings.onedrive_enabled ? "Reautorizar OneDrive" : "Conectar OneDrive"}
            </button>
            <label className="toggle-backup-label">
              <input
                type="checkbox"
                checked={onedriveEnabled}
                onChange={(e) => setOnedriveEnabled(e.target.checked)}
              />
              <span>Ativar OneDrive Backup</span>
            </label>
          </div>
        </section>

        {/* Auto Backup Options */}
        <section className="settings-section-card">
          <div className="section-header">
            <Clock size={20} className="section-icon" />
            <h2>Opções de Backup Automático</h2>
          </div>

          <div className="form-group-field short">
            <label>Frequência do Backup Automático (Minutos)</label>
            <input
              type="number"
              min="5"
              value={backupFrequencyMins}
              onChange={(e) => setBackupFrequencyMins(parseInt(e.target.value) || 60)}
            />
            <p className="field-tip">
              Define o intervalo de tempo em que a aplicação salvará silenciosamente os
              dados na nuvem.
            </p>
          </div>
        </section>

        <div className="settings-actions-footer">
          <button type="submit" className="btn-primary">
            Salvar Preferências
          </button>
          <button
            type="button"
            className="btn-secondary"
            onClick={onManualBackup}
            disabled={isBackingUp}
          >
            <RefreshCw size={14} className={isBackingUp ? "spin" : ""} />
            <span>Fazer Backup Agora</span>
          </button>
        </div>
      </form>

      {/* Manual Backups Management */}
      <section className="settings-section-card danger-zone">
        <div className="section-header">
          <Database size={20} className="section-icon text-rose" />
          <h2>Restauração de Backup Manual</h2>
        </div>
        <p>
          Você pode forçar a restauração dos seus dados a partir dos arquivos salvos em
          nuvem a qualquer momento.
        </p>

        <div className="restore-actions">
          <button
            className="btn-secondary danger"
            onClick={() => onRestoreBackup("gdrive")}
            disabled={isRestoring || !settings.gdrive_enabled}
          >
            Restaurar do Google Drive
          </button>
          <button
            className="btn-secondary danger"
            onClick={() => onRestoreBackup("onedrive")}
            disabled={isRestoring || !settings.onedrive_enabled}
          >
            Restaurar do OneDrive
          </button>
        </div>
      </section>

      {backupReport && (
        <section className="settings-section-card">
          <h3>Relatório do Último Backup Manual</h3>
          <div className="report-logs">
            {!backupReport.onedrive.enabled && !backupReport.gdrive.enabled ? (
              <p className="text-rose">Nenhum provedor de nuvem está configurado ou ativado para backup.</p>
            ) : (
              <>
                {backupReport.onedrive.enabled && (
                  <p>
                    <strong>OneDrive:</strong>{" "}
                    {backupReport.onedrive.success ? (
                      <span className="text-emerald">Sucesso</span>
                    ) : (
                      <span className="text-rose">
                        Falhou ({backupReport.onedrive.error_message})
                      </span>
                    )}
                  </p>
                )}
                {backupReport.gdrive.enabled && (
                  <p>
                    <strong>Google Drive:</strong>{" "}
                    {backupReport.gdrive.success ? (
                      <span className="text-emerald">Sucesso</span>
                    ) : (
                      <span className="text-rose">
                        Falhou ({backupReport.gdrive.error_message})
                      </span>
                    )}
                  </p>
                )}
              </>
            )}
          </div>
        </section>
      )}
    </div>
  );
};
