import React, { useState, useEffect } from "react";
import { Cloud, Clock, RefreshCw, Database, ShieldCheck, ChevronDown, ChevronUp } from "lucide-react";
import { AppSettings } from "../types";
import "./Settings.css";

interface SettingsProps {
  settings: AppSettings;
  onSaveSettings: (settings: AppSettings) => void;
  onConnectProvider: (provider: "gdrive" | "onedrive", clientId?: string, clientSecret?: string) => void;
  onRestoreBackup: (provider: string) => void;
  isRestoring: boolean;
  isBackingUp: boolean;
  backupReport: any;
  onManualBackup: () => void;
}

interface ProviderCardProps {
  name: string;
  provider: "gdrive" | "onedrive";
  enabled: boolean;
  onToggle: (enabled: boolean) => void;
  onConnect: () => void;
  clientId: string;
  setClientId: (val: string) => void;
  clientSecret: string;
  setClientSecret: (val: string) => void;
  isPkce?: boolean;
}

const ProviderSection: React.FC<ProviderCardProps> = ({
  name,
  provider,
  enabled,
  onToggle,
  onConnect,
  clientId,
  setClientId,
  clientSecret,
  setClientSecret,
  isPkce = false,
}) => {
  const [showAdvanced, setShowAdvanced] = useState(false);

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
            Autenticação segura via <strong>PKCE</strong> (sem segredo exposto) e armazenamento isolado na pasta interna (<strong>appDataFolder</strong>).
          </span>
        </div>
      )}

      <div className="provider-status-row">
        <div className={`status-badge ${enabled ? "connected" : "disconnected"}`}>
          {enabled ? "Habilitado e Conectado" : "Não Conectado"}
        </div>
        <button type="button" className="btn-secondary" onClick={onConnect}>
          {enabled ? `Reconectar ${name}` : `Conectar ${name}`}
        </button>
        <label className="toggle-backup-label">
          <input type="checkbox" checked={enabled} onChange={(e) => onToggle(e.target.checked)} />
          <span>Ativar Backup Automático</span>
        </label>
      </div>

      <div className="advanced-toggle-wrapper">
        <button
          type="button"
          className="btn-text-toggle"
          onClick={() => setShowAdvanced(!showAdvanced)}
        >
          {showAdvanced ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
          <span>Configurações Avançadas (Chaves Personalizadas)</span>
        </button>
      </div>

      {showAdvanced && (
        <div className="form-group-row advanced-fields">
          <div className="form-group-field">
            <label>Client ID {provider === "gdrive" ? "(Opcional - compilado no Rust)" : ""}</label>
            <input
              type="text"
              placeholder={provider === "gdrive" ? "Padrão embutido no backend" : "Ex: 5d1345a-cf2a-43d2"}
              value={clientId}
              onChange={(e) => setClientId(e.target.value)}
            />
          </div>
          <div className="form-group-field">
            <label>Client Secret (Opcional)</label>
            <input
              type="password"
              placeholder={provider === "gdrive" ? "Não obrigatório com PKCE" : "••••••••••••••••"}
              value={clientSecret}
              onChange={(e) => setClientSecret(e.target.value)}
            />
          </div>
        </div>
      )}
    </section>
  );
};

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
    onSaveSettings({
      ...settings,
      gdrive_client_id: gdriveClientId.trim() || undefined,
      gdrive_client_secret: gdriveClientSecret.trim() || undefined,
      onedrive_client_id: onedriveClientId.trim() || undefined,
      onedrive_client_secret: onedriveClientSecret.trim() || undefined,
      backup_frequency_mins: backupFrequencyMins,
      gdrive_enabled: gdriveEnabled,
      onedrive_enabled: onedriveEnabled,
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
          enabled={gdriveEnabled}
          onToggle={setGdriveEnabled}
          onConnect={() => onConnectProvider("gdrive", gdriveClientId, gdriveClientSecret)}
          clientId={gdriveClientId}
          setClientId={setGdriveClientId}
          clientSecret={gdriveClientSecret}
          setClientSecret={setGdriveClientSecret}
          isPkce
        />

        <ProviderSection
          name="OneDrive"
          provider="onedrive"
          enabled={onedriveEnabled}
          onToggle={setOnedriveEnabled}
          onConnect={() => onConnectProvider("onedrive", onedriveClientId, onedriveClientSecret)}
          clientId={onedriveClientId}
          setClientId={setOnedriveClientId}
          clientSecret={onedriveClientSecret}
          setClientSecret={setOnedriveClientSecret}
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
