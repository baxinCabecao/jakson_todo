import React, { useState, useEffect } from "react";
import { Cloud, Clock, RefreshCw, Database, ShieldCheck } from "lucide-react";
import { AppSettings } from "../types";
import "./Settings.css";

interface SettingsProps {
  settings: AppSettings;
  onSaveSettings: (settings: AppSettings) => void;
  onConnectProvider: (provider: "gdrive" | "onedrive") => void;
  onDisconnectProvider: (provider: "gdrive" | "onedrive") => void;
  onRestoreBackup: (provider: string) => void;
  isRestoring: boolean;
  isBackingUp: boolean;
  backupReport: any;
  onManualBackup: () => void;
}

interface ProviderCardProps {
  name: string;
  provider: "gdrive" | "onedrive";
  isConnected: boolean;
  enabled: boolean;
  onToggle: (enabled: boolean) => void;
  onConnect: () => void;
  onDisconnect: () => void;
  isPkce?: boolean;
}

const ProviderSection: React.FC<ProviderCardProps> = ({
  name,
  isConnected,
  enabled,
  onToggle,
  onConnect,
  onDisconnect,
  isPkce = false,
}) => {
  const getBadgeClass = () => {
    if (!isConnected) return "disconnected";
    return enabled ? "connected" : "paused";
  };

  const getBadgeText = () => {
    if (!isConnected) return "Não Conectado";
    return enabled ? "Conectado e Ativo" : "Conectado (Backup Pausado)";
  };

  return (
    <section className="settings-section-card">
      <div className="section-header">
        <Cloud size={20} className="section-icon" />
        <h2>Integração com {name}</h2>
      </div>

      {isPkce && (
        <div className="provider-info-box">
          <ShieldCheck size={18} className="text-emerald" />
          <span>
            Autenticação segura e direta via <strong>PKCE</strong> com armazenamento isolado na pasta interna (<strong>appDataFolder</strong>).
          </span>
        </div>
      )}

      <div className="provider-status-row">
        <div className={`status-badge ${getBadgeClass()}`}>
          {getBadgeText()}
        </div>

        {isConnected ? (
          <>
            <button type="button" className="btn-secondary" onClick={onConnect}>
              Reconectar {name}
            </button>
            <button type="button" className="btn-secondary danger-text" onClick={onDisconnect}>
              Desconectar
            </button>
          </>
        ) : (
          <button type="button" className="btn-primary" onClick={onConnect}>
            Conectar {name}
          </button>
        )}

        <label className={`toggle-backup-label ${!isConnected ? "disabled" : ""}`}>
          <input
            type="checkbox"
            checked={enabled && isConnected}
            disabled={!isConnected}
            onChange={(e) => {
              if (!isConnected) {
                alert(`Conecte sua conta do ${name} primeiro antes de ativar o backup automático.`);
                return;
              }
              onToggle(e.target.checked);
            }}
          />
          <span>Ativar Backup Automático</span>
        </label>
      </div>
      {!isConnected && (
        <p className="field-tip mt-1">
          * Conecte sua conta do {name} acima para liberar o backup em nuvem.
        </p>
      )}
    </section>
  );
};

export const Settings: React.FC<SettingsProps> = ({
  settings,
  onSaveSettings,
  onConnectProvider,
  onDisconnectProvider,
  onRestoreBackup,
  isRestoring,
  isBackingUp,
  backupReport,
  onManualBackup,
}) => {
  const [backupFrequencyMins, setBackupFrequencyMins] = useState(60);
  const [gdriveEnabled, setGdriveEnabled] = useState(false);
  const [onedriveEnabled, setOnedriveEnabled] = useState(false);

  const isGdriveConnected = Boolean(settings.gdrive_refresh_token);
  const isOnedriveConnected = Boolean(settings.onedrive_refresh_token);

  useEffect(() => {
    setBackupFrequencyMins(settings.backup_frequency_mins);
    setGdriveEnabled(settings.gdrive_enabled && Boolean(settings.gdrive_refresh_token));
    setOnedriveEnabled(settings.onedrive_enabled && Boolean(settings.onedrive_refresh_token));
  }, [settings]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSaveSettings({
      ...settings,
      backup_frequency_mins: backupFrequencyMins,
      gdrive_enabled: gdriveEnabled && isGdriveConnected,
      onedrive_enabled: onedriveEnabled && isOnedriveConnected,
    });
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
        <ProviderSection
          name="Google Drive"
          provider="gdrive"
          isConnected={isGdriveConnected}
          enabled={gdriveEnabled}
          onToggle={setGdriveEnabled}
          onConnect={() => onConnectProvider("gdrive")}
          onDisconnect={() => onDisconnectProvider("gdrive")}
          isPkce
        />

        <ProviderSection
          name="OneDrive"
          provider="onedrive"
          isConnected={isOnedriveConnected}
          enabled={onedriveEnabled}
          onToggle={setOnedriveEnabled}
          onConnect={() => onConnectProvider("onedrive")}
          onDisconnect={() => onDisconnectProvider("onedrive")}
        />

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
              Define o intervalo em que a aplicação salvará silenciosamente os dados na nuvem.
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
        <p>Você pode forçar a restauração dos seus dados salvos em nuvem a qualquer momento.</p>

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
                      <span className="text-rose">Falhou ({backupReport.onedrive.error_message})</span>
                    )}
                  </p>
                )}
                {backupReport.gdrive.enabled && (
                  <p>
                    <strong>Google Drive:</strong>{" "}
                    {backupReport.gdrive.success ? (
                      <span className="text-emerald">Sucesso</span>
                    ) : (
                      <span className="text-rose">Falhou ({backupReport.gdrive.error_message})</span>
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
