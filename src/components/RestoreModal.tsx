import React from "react";
import { AlertTriangle } from "lucide-react";
import { CloudBackupsCheck } from "../types";
import "./RestoreModal.css";

interface RestoreModalProps {
  isOpen: boolean;
  onClose: () => void;
  backupCheck: CloudBackupsCheck | null;
  onRestore: (provider: string) => void;
  isRestoring: boolean;
}

export const RestoreModal: React.FC<RestoreModalProps> = ({
  isOpen,
  onClose,
  backupCheck,
  onRestore,
  isRestoring,
}) => {
  if (!isOpen || !backupCheck) return null;

  return (
    <div className="modal-backdrop z-50">
      <div className="modal-card alert-card animate-fade-in">
        <div className="modal-header">
          <h2 className="text-warning flex items-center gap-2">
            <AlertTriangle size={20} />
            <span>Atualização da Nuvem Disponível</span>
          </h2>
        </div>
        <div className="modal-body">
          <p>
            Detectamos dados mais recentes na nuvem gravados por outro dispositivo.
          </p>

          <div className="backup-comparison-box">
            <p>
              <strong>Provedor Recomendado:</strong> {backupCheck.recommended_provider === "gdrive" ? "Google Drive" : backupCheck.recommended_provider === "onedrive" ? "OneDrive" : backupCheck.recommended_provider}
            </p>
            <p>
              <strong>Última Sincronização Local:</strong>{" "}
              {backupCheck.local_last_modified
                ? new Date(backupCheck.local_last_modified).toLocaleString()
                : "Sem registro anterior"}
            </p>

            {backupCheck.gdrive.exists && backupCheck.gdrive.last_modified && (
              <p>
                <strong>Google Drive:</strong>{" "}
                {new Date(backupCheck.gdrive.last_modified).toLocaleString()}
              </p>
            )}

            {backupCheck.onedrive.exists && backupCheck.onedrive.last_modified && (
              <p>
                <strong>OneDrive:</strong>{" "}
                {new Date(backupCheck.onedrive.last_modified).toLocaleString()}
              </p>
            )}
          </div>

          <p className="warning-note">
            A sincronização irá mesclar os dados de forma inteligente (sem apagar suas alterações locais não sincronizadas). Deseja atualizar agora?
          </p>
        </div>
        <div className="modal-footer">
          <button className="btn-secondary" onClick={onClose} disabled={isRestoring}>
            Agora Não
          </button>
          <button
            className="btn-primary"
            onClick={() => onRestore(backupCheck.recommended_provider || "gdrive")}
            disabled={isRestoring}
          >
            {isRestoring
              ? "Sincronizando..."
              : `Sincronizar com ${backupCheck.recommended_provider === "onedrive" ? "OneDrive" : "Google Drive"}`}
          </button>
        </div>
      </div>
    </div>
  );
};
