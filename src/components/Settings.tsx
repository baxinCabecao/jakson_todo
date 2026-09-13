import React, { useState, useEffect } from "react";
import { Cloud, Clock, RefreshCw, ShieldCheck } from "lucide-react";
import { AppSettings } from "../types";
import "./Settings.css";

interface SettingsProps {
  settings: AppSettings;
  onSaveSettings: (settings: AppSettings) => void;
  onConnectProvider: (provider: "gdrive" | "onedrive") => void;
  onDisconnectProvider: (provider: "gdrive" | "onedrive") => void;
  onRestoreBackup?: (provider: string) => void;
  onRestoreSafetyBackup: (provider: string) => void;
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
    return enabled ? "Conectado e Sincronizando" : "Conectado (Sincronização Pausada)";
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
            Autenticação segura via <strong>PKCE</strong> na pasta isolada (<strong>appDataFolder</strong>).
          </span>
        </div>
      )}

      <div className="provider-status-row">
        <div className={`status-badge ${getBadgeClass()}`}>{getBadgeText()}</div>

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
                alert(`Conecte sua conta do ${name} primeiro antes de ativar a sincronização.`);
                return;
              }
              onToggle(e.target.checked);
            }}
          />
          <span>Sincronização Automática</span>
        </label>
      </div>
      {!isConnected && (
        <p className="field-tip mt-1">* Conecte sua conta do {name} acima para sincronizar entre seus dispositivos.</p>
      )}
    </section>
  );
};

export const Settings: React.FC<SettingsProps> = ({
  settings,
  onSaveSettings,
  onConnectProvider,
  onDisconnectProvider,
  onRestoreSafetyBackup,
  isRestoring,
  isBackingUp,
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

  const formatDateTime = (isoStr?: string) => {
    if (!isoStr) return "Nenhuma sincronização registrada ainda";
    try {
      const d = new Date(isoStr);
      return d.toLocaleString("pt-BR", {
        day: "2-digit",
        month: "2-digit",
        year: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return isoStr;
    }
  };

  return (
    <div className="tab-pane animate-fade-in">
      <header className="content-header">
        <div>
          <h1>Configurações da Aplicação</h1>
          <p>Gerencie provedores de nuvem, sincronização entre dispositivos e cópias de recuperação.</p>
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

        {/* Auto Sync Frequency */}
        <section className="settings-section-card">
          <div className="section-header">
            <Clock size={20} className="section-icon" />
            <h2>Opções de Sincronização em Segundo Plano</h2>
          </div>

          <div className="form-group-field short">
            <label>Intervalo de Verificação Periódica (Minutos)</label>
            <input
              type="number"
              min="1"
              value={backupFrequencyMins}
              onChange={(e) => setBackupFrequencyMins(parseInt(e.target.value) || 60)}
            />
            <p className="field-tip">
              Intervalo para verificação em segundo plano se há atualizações de outros dispositivos na nuvem.
            </p>
          </div>
        </section>

        <div className="settings-actions-footer">
          <button type="submit" className="btn-primary">
            Salvar Preferências
          </button>
        </div>
      </form>

      {/* Real-Time Two-Way Sync Panel */}
      <section className="settings-section-card">
        <div className="section-header">
          <RefreshCw size={20} className="section-icon text-emerald" />
          <h2>Sincronização entre Dispositivos</h2>
        </div>
        <p>
          Suas tarefas e notas são sincronizadas de forma bidirecional e não destrutiva. Modificações em outros
          computadores ou celulares são unificadas sem perda de dados locais.
        </p>
        <p className="field-tip mt-1">
          <strong>Última sincronização:</strong> {formatDateTime(settings.last_backup_time)}
        </p>

        <div className="mt-2">
          <button
            type="button"
            className="btn-secondary"
            onClick={onManualBackup}
            disabled={isBackingUp || (!settings.gdrive_enabled && !settings.onedrive_enabled)}
          >
            <RefreshCw size={14} className={isBackingUp ? "spin" : ""} />
            <span>{isBackingUp ? "Sincronizando..." : "Sincronizar Agora"}</span>
          </button>
        </div>
      </section>

      {/* 24-Hour Safety Snapshot Restore */}
      <section className="settings-section-card">
        <div className="section-header">
          <ShieldCheck size={20} className="section-icon text-amber" />
          <h2>Cópia de Segurança de 24 Horas (Recuperação de Desastres)</h2>
        </div>
        <p>
          Snapshot estático mantido e rotacionado a cada 24 horas para recuperação caso você tenha excluído tarefas ou notas por acidente.
        </p>
        <p className="field-tip">
          <strong>Último snapshot:</strong> {formatDateTime(settings.last_safety_backup_time)}
        </p>

        <div className="restore-actions mt-1">
          <button
            type="button"
            className="btn-secondary danger"
            onClick={() => onRestoreSafetyBackup("gdrive")}
            disabled={isRestoring || !settings.gdrive_enabled}
          >
            Restaurar Snapshot 24h (GDrive)
          </button>
          <button
            type="button"
            className="btn-secondary danger"
            onClick={() => onRestoreSafetyBackup("onedrive")}
            disabled={isRestoring || !settings.onedrive_enabled}
          >
            Restaurar Snapshot 24h (OneDrive)
          </button>
        </div>
      </section>
    </div>
  );
};
