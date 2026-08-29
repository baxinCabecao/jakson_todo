import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { Plus, Menu } from "lucide-react";

import "./App.css"; // CRITICAL: Import the layout styles!
import { Task, AppSettings, CloudBackupsCheck } from "./types";
import { Sidebar } from "./components/Sidebar";
import { Dashboard } from "./components/Dashboard";
import { FiltersBar } from "./components/FiltersBar";
import { TasksFeed } from "./components/TasksFeed";
import { TaskModal } from "./components/TaskModal";
import { Settings } from "./components/Settings";
import { RestoreModal } from "./components/RestoreModal";
import { EisenhowerMatrix } from "./components/EisenhowerMatrix";

function App() {
  // Navigation tabs
  const [activeTab, setActiveTab] = useState<"tasks" | "dashboard" | "settings" | "eisenhower">("tasks");

  // State
  const [tasks, setTasks] = useState<Task[]>([]);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(() => {
    return localStorage.getItem("sidebar-collapsed") === "true";
  });

  const handleToggleSidebar = (collapsed: boolean) => {
    setIsSidebarCollapsed(collapsed);
    localStorage.setItem("sidebar-collapsed", String(collapsed));
  };
  const [settings, setSettings] = useState<AppSettings>({
    onedrive_enabled: false,
    gdrive_enabled: false,
    backup_frequency_mins: 60,
  });

  // Filters & Sorting States
  const [filterStatus, setFilterStatus] = useState<string>("all");
  const [filterPriority, setFilterPriority] = useState<string>("all");
  const [filterDueDate, setFilterDueDate] = useState<string>("all"); // "all", "today", "tomorrow", "overdue"
  const [showCompleted, setShowCompleted] = useState<boolean>(false);
  const [sortBy, setSortBy] = useState<"due_date" | "priority" | "title">("due_date");
  const [searchQuery, setSearchQuery] = useState<string>("");

  // Modal / Form States
  const [isTaskModalOpen, setIsTaskModalOpen] = useState(false);
  const [editingTask, setEditingTask] = useState<Task | null>(null);

  // Cloud Backup Alert State
  const [backupCheck, setBackupCheck] = useState<CloudBackupsCheck | null>(null);
  const [showRestoreModal, setShowRestoreModal] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);
  const [backupReport, setBackupReport] = useState<any>(null);
  const [isBackingUp, setIsBackingUp] = useState(false);

  // Font Size State (Default 13px, persisted in localStorage)
  const [fontSize, setFontSize] = useState<number>(() => {
    const saved = localStorage.getItem("todo-font-size");
    return saved ? parseInt(saved, 10) : 13;
  });

  // Apply font size globally via CSS Variable
  useEffect(() => {
    document.documentElement.style.setProperty("--app-font-size", `${fontSize}px`);
    localStorage.setItem("todo-font-size", fontSize.toString());
  }, [fontSize]);

  // Layout columns state (2 or 3, persisted in localStorage)
  const [columns, setColumns] = useState<2 | 3>(() => {
    const saved = localStorage.getItem("todo-layout-columns");
    return saved === "3" ? 3 : 2;
  });

  // Persist layout columns selection
  useEffect(() => {
    localStorage.setItem("todo-layout-columns", columns.toString());
  }, [columns]);

  // Load tasks and settings on startup
  useEffect(() => {
    loadTasks();
    loadSettings();
    checkCloudBackups();

    // Listen for OAuth success/error events from Rust backend
    const unlistenSuccess = listen<string>("oauth-success", (event) => {
      alert(`Conexão com ${event.payload === "gdrive" ? "Google Drive" : "OneDrive"} realizada com sucesso!`);
      loadSettings();
    });

    const unlistenError = listen<string>("oauth-error", (event) => {
      alert(`Erro na autenticação: ${event.payload}`);
    });

    // Listen for OAuth deep link callbacks (Mobile & Desktop)
    const unlistenDeepLink = onOpenUrl((urls) => {
      for (const url of urls) {
        if (url.includes("code=") || url.includes("error=")) {
          invoke("handle_oauth_url", { url }).catch((err) => {
            console.error("Erro ao processar callback OAuth via Deep Link:", err);
          });
        }
      }
    });

    return () => {
      unlistenSuccess.then((f) => f());
      unlistenError.then((f) => f());
      unlistenDeepLink.then((f) => f());
    };
  }, []);

  const loadTasks = async () => {
    try {
      const res = await invoke<Task[]>("get_tasks");
      setTasks(res);
    } catch (e) {
      console.error("Erro ao buscar tarefas:", e);
    }
  };

  const loadSettings = async () => {
    try {
      const res = await invoke<AppSettings>("get_settings");
      setSettings(res);
    } catch (e) {
      console.error("Erro ao buscar configurações:", e);
    }
  };

  const checkCloudBackups = async () => {
    try {
      const res = await invoke<CloudBackupsCheck>("check_backups");
      setBackupCheck(res);
      if (res.newer_backup_available) {
        setShowRestoreModal(true);
      }
    } catch (e) {
      console.error("Erro ao checar backups na nuvem:", e);
    }
  };

  const handleRestoreBackup = async (provider: string) => {
    setIsRestoring(true);
    try {
      await invoke("restore_backup", { provider });
      alert("Banco de dados restaurado com sucesso do backup!");
      setShowRestoreModal(false);
      loadTasks();
      loadSettings();
    } catch (e) {
      alert(`Erro ao restaurar backup: ${e}`);
    } finally {
      setIsRestoring(false);
    }
  };

  const handleManualBackup = async () => {
    setIsBackingUp(true);
    try {
      const report = await invoke<any>("trigger_backup");
      setBackupReport(report);
      loadSettings();
    } catch (e) {
      alert(`Erro ao disparar backup: ${e}`);
    } finally {
      setIsBackingUp(false);
    }
  };

  // Helper date status mapping
  const getDueStatus = (dueDate?: string) => {
    if (!dueDate) return "none";
    const todayStr = new Date().toISOString().split("T")[0];

    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    const tomorrowStr = tomorrow.toISOString().split("T")[0];

    if (dueDate < todayStr) return "overdue";
    if (dueDate === todayStr) return "today";
    if (dueDate === tomorrowStr) return "tomorrow";
    return "future";
  };

  // Sort priorities helper
  const priorityWeight = (p: string) => {
    switch (p.toLowerCase()) {
      case "high": return 3;
      case "medium": return 2;
      case "low": return 1;
      default: return 0;
    }
  };

  // Computed and sorted tasks list
  const filteredAndSortedTasks = useMemo(() => {
    return tasks
      .map(task => {
        // Filter subtasks inside the task if showCompleted is false
        if (!showCompleted && task.subtasks) {
          return {
            ...task,
            subtasks: task.subtasks.filter(sub => !sub.completed)
          };
        }
        return task;
      })
      .filter((task) => {
        // Toggle completed tasks filter
        if (!showCompleted && task.status === "completed") {
          return false;
        }

        // Status filter
        if (filterStatus !== "all" && task.status !== filterStatus) {
          return false;
        }

        // Priority filter
        if (filterPriority !== "all" && task.priority !== filterPriority) {
          return false;
        }

        // Due date filter
        if (filterDueDate !== "all") {
          const status = getDueStatus(task.due_date);
          if (filterDueDate === "overdue" && status !== "overdue") return false;
          if (filterDueDate === "today" && status !== "today") return false;
          if (filterDueDate === "tomorrow" && status !== "tomorrow") return false;
        }

        // Search query filter
        if (searchQuery.trim() !== "") {
          const query = searchQuery.toLowerCase();
          const titleMatch = task.title.toLowerCase().includes(query);
          const descMatch = task.description?.toLowerCase().includes(query) || false;
          return titleMatch || descMatch;
        }

        return true;
      })
      .sort((a, b) => {
        if (sortBy === "due_date") {
          if (!a.due_date && b.due_date) return 1;
          if (a.due_date && !b.due_date) return -1;
          if (!a.due_date && !b.due_date) {
            return priorityWeight(b.priority) - priorityWeight(a.priority);
          }
          if (a.due_date && b.due_date) {
            if (a.due_date !== b.due_date) {
              return a.due_date.localeCompare(b.due_date);
            }
            return priorityWeight(b.priority) - priorityWeight(a.priority);
          }
        } else if (sortBy === "priority") {
          const weightDiff = priorityWeight(b.priority) - priorityWeight(a.priority);
          if (weightDiff !== 0) return weightDiff;
          if (!a.due_date && b.due_date) return 1;
          if (a.due_date && !b.due_date) return -1;
          if (a.due_date && b.due_date) return a.due_date.localeCompare(b.due_date);
        } else if (sortBy === "title") {
          return a.title.localeCompare(b.title);
        }
        return 0;
      });
  }, [tasks, filterStatus, filterPriority, filterDueDate, showCompleted, sortBy, searchQuery]);

  // Open modal for task creation
  const openCreateModal = () => {
    setEditingTask(null);
    setIsTaskModalOpen(true);
  };

  // Open modal for task editing
  const openEditModal = (task: Task) => {
    setEditingTask(task);
    setIsTaskModalOpen(true);
  };

  // Save or Update task
  const handleSaveTask = async (taskPayload: Task) => {
    try {
      if (editingTask) {
        await invoke("update_task", { task: taskPayload });
      } else {
        await invoke("create_task", { task: taskPayload });
      }
      setIsTaskModalOpen(false);
      loadTasks();
    } catch (e) {
      alert(`Erro ao salvar tarefa: ${e}`);
    }
  };

  // Delete task
  const handleDeleteTask = async (id: number) => {
    if (!confirm("Tem certeza que deseja excluir esta tarefa?")) return;
    try {
      await invoke("delete_task", { id });
      loadTasks();
    } catch (e) {
      alert(`Erro ao excluir tarefa: ${e}`);
    }
  };

  // Toggle task completion status
  const handleToggleTaskCompleted = async (task: Task) => {
    const newStatus = task.status === "completed" ? "todo" : "completed";
    let subtasks = task.subtasks || [];
    if (newStatus === "completed") {
      subtasks = subtasks.map(s => ({ ...s, completed: true }));
    }

    const updated: Task = {
      ...task,
      status: newStatus,
      subtasks,
    };

    try {
      await invoke("update_task", { task: updated });
      loadTasks();
    } catch (e) {
      console.error(e);
    }
  };

  // Toggle subtask completion status
  const handleToggleSubtask = async (task: Task, subtaskIndex: number) => {
    if (!task.subtasks) return;
    const subtasks = [...task.subtasks];
    subtasks[subtaskIndex].completed = !subtasks[subtaskIndex].completed;

    const updated: Task = {
      ...task,
      subtasks,
    };

    try {
      await invoke("update_task", { task: updated });
      loadTasks();
    } catch (e) {
      console.error(e);
    }
  };

  // Update task due date (autosave)
  const handleUpdateDueDate = async (task: Task, newDueDate: string | undefined) => {
    const updated: Task = {
      ...task,
      due_date: newDueDate,
    };
    try {
      await invoke("update_task", { task: updated });
      loadTasks();
    } catch (e) {
      console.error(e);
      alert(`Erro ao atualizar data de vencimento: ${e}`);
    }
  };

  // Save Settings
  const handleSaveSettings = async (updatedSettings: AppSettings) => {
    try {
      await invoke("save_settings", { settings: updatedSettings });
      alert("Configurações salvas com sucesso!");
      loadSettings();
    } catch (e) {
      alert(`Erro ao salvar configurações: ${e}`);
    }
  };

  // Trigger Cloud Authorize flow
  const handleConnectProvider = async (provider: "gdrive" | "onedrive", clientId?: string, clientSecret?: string) => {
    try {
      await invoke("start_oauth", {
        provider,
        clientId: clientId?.trim() || null,
        clientSecret: clientSecret?.trim() || null,
      });
    } catch (e) {
      alert(`Erro ao iniciar fluxo OAuth: ${e}`);
    }
  };

  return (
    <div className={`app-container ${isSidebarCollapsed ? "sidebar-collapsed" : ""}`}>
      {/* Sidebar Navigation */}
      <Sidebar
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        onManualBackup={handleManualBackup}
        isBackingUp={isBackingUp}
        lastBackupTime={settings.last_backup_time}
        fontSize={fontSize}
        setFontSize={setFontSize}
        isCollapsed={isSidebarCollapsed}
        setIsCollapsed={handleToggleSidebar}
      />

      {/* Main Panel */}
      <main className="main-content">
        {isSidebarCollapsed && (
          <button
            type="button"
            className="btn-sidebar-toggle-open"
            onClick={() => handleToggleSidebar(false)}
            title="Mostrar Menu"
          >
            <Menu size={18} />
          </button>
        )}
        {activeTab === "tasks" && (
          <div className="tab-pane animate-fade-in">
            <header className="content-header">
              <div>
                <h1>Lista de Tarefas</h1>
                <p>Gerencie, filtre e acompanhe seu fluxo de trabalho diário.</p>
              </div>
              <button className="btn-primary" onClick={openCreateModal}>
                <Plus size={18} />
                <span>Nova Tarefa</span>
              </button>
            </header>

            {/* Filters Bar */}
            <FiltersBar
              searchQuery={searchQuery}
              setSearchQuery={setSearchQuery}
              filterStatus={filterStatus}
              setFilterStatus={setFilterStatus}
              filterPriority={filterPriority}
              setFilterPriority={setFilterPriority}
              filterDueDate={filterDueDate}
              setFilterDueDate={setFilterDueDate}
              sortBy={sortBy}
              setSortBy={setSortBy}
              showCompleted={showCompleted}
              setShowCompleted={setShowCompleted}
              columns={columns}
              setColumns={setColumns}
            />

            {/* Tasks Feed */}
            <TasksFeed
              tasks={filteredAndSortedTasks}
              columns={columns}
              getDueStatus={getDueStatus}
              onToggleCompleted={handleToggleTaskCompleted}
              onToggleSubtask={handleToggleSubtask}
              onEdit={openEditModal}
              onDelete={handleDeleteTask}
              onUpdateDueDate={handleUpdateDueDate}
            />
          </div>
        )}

        {activeTab === "eisenhower" && (
          <EisenhowerMatrix
            tasks={tasks}
            getDueStatus={getDueStatus}
            onEdit={openEditModal}
            onToggleCompleted={handleToggleTaskCompleted}
            onOpenCreateModal={openCreateModal}
            showCompleted={showCompleted}
            setShowCompleted={setShowCompleted}
            onUpdateDueDate={handleUpdateDueDate}
          />
        )}

        {activeTab === "dashboard" && (
          <Dashboard tasks={tasks} getDueStatus={getDueStatus} />
        )}

        {activeTab === "settings" && (
          <Settings
            settings={settings}
            onSaveSettings={handleSaveSettings}
            onConnectProvider={handleConnectProvider}
            onRestoreBackup={handleRestoreBackup}
            isRestoring={isRestoring}
            isBackingUp={isBackingUp}
            backupReport={backupReport}
            onManualBackup={handleManualBackup}
          />
        )}
      </main>

      {/* Task Creation & Editing Modal */}
      <TaskModal
        isOpen={isTaskModalOpen}
        onClose={() => setIsTaskModalOpen(false)}
        editingTask={editingTask}
        onSave={handleSaveTask}
      />

      {/* Startup Cloud Restore Alert Modal */}
      <RestoreModal
        isOpen={showRestoreModal}
        onClose={() => setShowRestoreModal(false)}
        backupCheck={backupCheck}
        onRestore={handleRestoreBackup}
        isRestoring={isRestoring}
      />
    </div>
  );
}

export default App;
