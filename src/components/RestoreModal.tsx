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
            <span>Backup Mais Recente Encontrado!</span>
          </h2>
        </div>
        <div className="modal-body">
          <p>
            A aplicação detectou que existe um backup mais novo na nuvem em comparação com
            os dados locais.
          </p>

          <div className="backup-comparison-box">
            <p>
              <strong>Provedor Recomendado:</strong> {backupCheck.recommended_provider}
            </p>
            <p>
              <strong>Última Modificação Local:</strong>{" "}
              {backupCheck.local_last_modified
                ? new Date(backupCheck.local_last_modified).toLocaleString()
                : "Sem dados"}
            </p>

            {backupCheck.gdrive.exists && backupCheck.gdrive.last_modified && (
              <p>
                <strong>Google Drive Backup:</strong>{" "}
                {new Date(backupCheck.gdrive.last_modified).toLocaleString()}
              </p>
            )}

            {backupCheck.onedrive.exists && backupCheck.onedrive.last_modified && (
              <p>
                <strong>OneDrive Backup:</strong>{" "}
                {new Date(backupCheck.onedrive.last_modified).toLocaleString()}
              </p>
            )}
          </div>

          <p className="warning-note">
            A restauração irá substituir os seus dados locais atuais. Deseja prosseguir
            com a restauração?
          </p>
        </div>
        <div className="modal-footer">
          <button className="btn-secondary" onClick={onClose} disabled={isRestoring}>
            Manter Dados Locais
          </button>
          <button
            className="btn-primary"
            onClick={() => onRestore(backupCheck.recommended_provider || "gdrive")}
            disabled={isRestoring}
          >
            {isRestoring
              ? "Restaurando..."
              : `Restaurar do ${backupCheck.recommended_provider}`}
          </button>
        </div>
      </div>
    </div>
  );
};
