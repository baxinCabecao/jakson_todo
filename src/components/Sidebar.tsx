import React from "react";
import { CheckSquare, Briefcase, Settings as SettingsIcon, Cloud, ListTodo, Grid, ChevronLeft } from "lucide-react";
import "./Sidebar.css";

interface SidebarProps {
  activeTab: "tasks" | "dashboard" | "settings" | "eisenhower";
  setActiveTab: (tab: "tasks" | "dashboard" | "settings" | "eisenhower") => void;
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
  return (
    <aside className={`sidebar ${isCollapsed ? "collapsed" : ""}`}>
      <div className="sidebar-brand">
        <ListTodo className="sidebar-logo-icon" size={24} />
        <h2>Jakson Todo</h2>
        <button
          type="button"
          className="btn-sidebar-collapse"
          onClick={() => setIsCollapsed(true)}
          title="Esconder Menu"
        >
          <ChevronLeft size={18} />
        </button>
      </div>

      <nav className="sidebar-nav">
        <button
          className={`nav-item ${activeTab === "tasks" ? "active" : ""}`}
          onClick={() => setActiveTab("tasks")}
        >
          <CheckSquare size={18} />
          <span>Minhas Tarefas</span>
        </button>
        <button
          className={`nav-item ${activeTab === "eisenhower" ? "active" : ""}`}
          onClick={() => setActiveTab("eisenhower")}
        >
          <Grid size={18} />
          <span>Matriz Eisenhower</span>
        </button>
        <button
          className={`nav-item ${activeTab === "dashboard" ? "active" : ""}`}
          onClick={() => setActiveTab("dashboard")}
        >
          <Briefcase size={18} />
          <span>Dashboard</span>
        </button>
        <button
          className={`nav-item ${activeTab === "settings" ? "active" : ""}`}
          onClick={() => setActiveTab("settings")}
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
        >
          <Cloud size={16} />
          <span>{isBackingUp ? "Backup..." : "Backup Nuvem"}</span>
        </button>
        {lastBackupTime && (
          <span className="last-backup-label">
            U. Backup: {new Date(lastBackupTime).toLocaleDateString()}
          </span>
        )}
      </div>
    </aside>
  );
};
