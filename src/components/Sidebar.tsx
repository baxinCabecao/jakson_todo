import React from "react";
import { CheckSquare, Briefcase, Settings as SettingsIcon, ListTodo, Grid, ChevronLeft, FileText, RefreshCw } from "lucide-react";
import "./Sidebar.css";

interface SidebarProps {
  activeTab: "tasks" | "notes" | "dashboard" | "settings" | "eisenhower";
  setActiveTab: (tab: "tasks" | "notes" | "dashboard" | "settings" | "eisenhower") => void;
  onManualBackup: () => void;
  isBackingUp: boolean;
  lastBackupTime?: string;
  fontSize: number;
  setFontSize: (size: number) => void;
  isCollapsed: boolean;
  setIsCollapsed: (collapsed: boolean) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeTab,
  setActiveTab,
  onManualBackup,
  isBackingUp,
  lastBackupTime,
  fontSize,
  setFontSize,
  isCollapsed,
  setIsCollapsed,
}) => {
  const handleNavClick = (tab: "tasks" | "notes" | "dashboard" | "settings" | "eisenhower") => {
    setActiveTab(tab);
    if (window.innerWidth <= 768) {
      setIsCollapsed(true);
    }
  };

  return (
    <aside className={`sidebar ${isCollapsed ? "collapsed" : ""}`}>
      <div className="sidebar-brand">
        <ListTodo className="sidebar-logo-icon" size={24} />
        <h2>Jakson ToDo</h2>
        <button
          type="button"
          className="btn-sidebar-collapse"
          onClick={() => setIsCollapsed(true)}
          title="Fechar Menu"
        >
          <ChevronLeft size={22} />
        </button>
      </div>

      <nav className="sidebar-nav">
        <button
          className={`nav-item ${activeTab === "tasks" ? "active" : ""}`}
          onClick={() => handleNavClick("tasks")}
        >
          <CheckSquare size={18} />
          <span>Minhas Tarefas</span>
        </button>
        <button
          className={`nav-item ${activeTab === "eisenhower" ? "active" : ""}`}
          onClick={() => handleNavClick("eisenhower")}
        >
          <Grid size={18} />
          <span>Matriz Eisenhower</span>
        </button>
        <button
          className={`nav-item ${activeTab === "notes" ? "active" : ""}`}
          onClick={() => handleNavClick("notes")}
        >
          <FileText size={18} />
          <span>Bloco de Notas</span>
        </button>
        <button
          className={`nav-item ${activeTab === "dashboard" ? "active" : ""}`}
          onClick={() => handleNavClick("dashboard")}
        >
          <Briefcase size={18} />
          <span>Dashboard</span>
        </button>
        <button
          className={`nav-item ${activeTab === "settings" ? "active" : ""}`}
          onClick={() => handleNavClick("settings")}
        >
          <SettingsIcon size={18} />
          <span>Configurações</span>
        </button>
      </nav>

      <div className="sidebar-footer">
        <div className="font-size-control">
          <span>Tamanho da Fonte: {fontSize}px</span>
          <input
            type="range"
            min="10"
            max="20"
            value={fontSize}
            onChange={(e) => setFontSize(parseInt(e.target.value))}
            className="font-size-slider"
          />
        </div>
        <button
          className="btn-backup-sidebar"
          onClick={onManualBackup}
          disabled={isBackingUp}
          title="Sincronizar tarefas e notas com a nuvem"
        >
          <RefreshCw size={15} className={isBackingUp ? "spin" : ""} />
          <span>{isBackingUp ? "Sincronizando..." : "Sincronizar"}</span>
        </button>
        {lastBackupTime && (
          <span className="last-backup-label">
            Sincronizado: {new Date(lastBackupTime).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
          </span>
        )}
      </div>
    </aside>
  );
};
