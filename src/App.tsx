import { useState, useEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onBackButtonPress } from "@tauri-apps/api/app";
import { onOpenUrl, getCurrent } from "@tauri-apps/plugin-deep-link";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import { Plus, Menu } from "lucide-react";

import "./App.css"; // CRITICAL: Import the layout styles!
import { Task, Note, AppSettings, CloudBackupsCheck, SyncResult } from "./types";
import { Sidebar } from "./components/Sidebar";
import { Dashboard } from "./components/Dashboard";
import { FiltersBar } from "./components/FiltersBar";
import { TasksFeed } from "./components/TasksFeed";
import { TaskModal } from "./components/TaskModal";
import { Settings } from "./components/Settings";
import { RestoreModal } from "./components/RestoreModal";
import { EisenhowerMatrix } from "./components/EisenhowerMatrix";
import { NotesFeed } from "./components/NotesFeed";
import { NoteModal } from "./components/NoteModal";

function App() {
  // Navigation tabs
  const [activeTab, setActiveTab] = useState<"tasks" | "notes" | "dashboard" | "settings" | "eisenhower">("tasks");

  // State
  const [tasks, setTasks] = useState<Task[]>([]);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(() => {
    const saved = localStorage.getItem("sidebar-collapsed");
    if (saved !== null) return saved === "true";
    return typeof window !== "undefined" && window.innerWidth <= 768;
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

  // Task Modal / Form States
  const [isTaskModalOpen, setIsTaskModalOpen] = useState(false);
  const [editingTask, setEditingTask] = useState<Task | null>(null);

  // Notes States
  const [notes, setNotes] = useState<Note[]>([]);
  const [isNoteModalOpen, setIsNoteModalOpen] = useState(false);
  const [editingNote, setEditingNote] = useState<Note | null>(null);

  // Cloud Backup Alert State
  const [backupCheck, setBackupCheck] = useState<CloudBackupsCheck | null>(null);
  const [showRestoreModal, setShowRestoreModal] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);
  const [backupReport, setBackupReport] = useState<any>(null);
  const [isBackingUp, setIsBackingUp] = useState(false);

  // Back navigation & exit state (Android & Desktop)
  const [exitToastVisible, setExitToastVisible] = useState(false);
  const lastBackPressTimeRef = useRef(0);
  const toastTimeoutRef = useRef<any>(null);

  // Debounced auto-sync timer ref
  const autoSyncTimerRef = useRef<any>(null);

  const triggerDebouncedSync = () => {
    if (autoSyncTimerRef.current) {
      clearTimeout(autoSyncTimerRef.current);
    }
    autoSyncTimerRef.current = setTimeout(async () => {
      try {
        const res = await invoke<SyncResult>("auto_sync");
        console.log("[App] Sincronização inteligente:", res);
        if (res.tasks_pulled || res.notes_pulled || res.action === "restored") {
          await loadTasks();
          await loadNotes();
        }
        await loadSettings();
      } catch (err) {
        console.error("[App] Erro na sincronização inteligente:", err);
      }
    }, 3000); // 3s debounce
  };

  // Keep latest navigation state accessible to global event listener without re-registering
  const navigationStateRef = useRef({
    isNoteModalOpen,
    isTaskModalOpen,
    showRestoreModal,
    isSidebarCollapsed,
    activeTab,
  });

  useEffect(() => {
    navigationStateRef.current = {
      isNoteModalOpen,
      isTaskModalOpen,
      showRestoreModal,
      isSidebarCollapsed,
      activeTab,
    };
  }, [isNoteModalOpen, isTaskModalOpen, showRestoreModal, isSidebarCollapsed, activeTab]);

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

  // Load tasks, notes and settings on startup
  useEffect(() => {
    loadTasks();
    loadNotes();
    loadSettings();

    // Trigger non-destructive two-way sync on startup
    invoke<SyncResult>("auto_sync")
      .then((res) => {
        console.log("[App] Sincronização de inicialização:", res);
        if (res.tasks_pulled || res.notes_pulled) {
          loadTasks();
          loadNotes();
        }
        loadSettings();
      })
      .catch((err) => {
        console.log("[App] Sincronização inicial em segundo plano:", err);
      });

    checkCloudBackups();

    // Listen for OAuth success/error events from Rust backend
    const unlistenSuccess = listen<string>("oauth-success", (event) => {
      alert(`Conexão com ${event.payload === "gdrive" ? "Google Drive" : "OneDrive"} realizada com sucesso!`);
      loadSettings();
    });

    const unlistenError = listen<string>("oauth-error", (event) => {
      alert(`Erro na autenticação: ${event.payload}`);
    });

    // Listen for automatic cloud synchronization updates from Rust
    const unlistenCloudSynced = listen<{ tasks_pulled?: number; notes_pulled?: number }>("cloud-synced", (event) => {
      console.log("[App] Dados sincronizados da nuvem:", event.payload);
      loadTasks();
      loadNotes();
      loadSettings();
    });

    const unlistenCloudRestored = listen<{ provider: string }>("cloud-restored", (event) => {
      console.log("[App] Dados sincronizados da nuvem via:", event.payload);
      loadTasks();
      loadNotes();
      loadSettings();
    });

    // Listen for OAuth deep link callbacks (Mobile & Desktop)
    const handleDeepLinkUrl = async (url: string) => {
      console.log("Deep link capturado:", url);
      if (url.includes("code=") || url.includes("error=")) {
        try {
          await invoke("handle_oauth_url", { url });
        } catch (err) {
          console.error("Erro ao processar callback OAuth via Deep Link:", err);
        }
      }
    };

    // Check cold-start deep links
    getCurrent()
      .then((urls) => {
        if (urls && urls.length > 0) {
          for (const url of urls) {
            handleDeepLinkUrl(url);
          }
        }
      })
      .catch((err) => {
        console.error("Erro ao verificar getCurrent:", err);
      });

    // Listen for deep links while app is running
    const unlistenDeepLink = onOpenUrl((urls) => {
      for (const url of urls) {
        handleDeepLinkUrl(url);
      }
    });

    // Request notification permissions for backup alerts (Android 13+ and Desktop)
    const ensureNotificationPermission = async () => {
      try {
        const granted = await isPermissionGranted();
        if (!granted) {
          await requestPermission();
        }
      } catch (err) {
        console.error("Erro ao verificar/solicitar permissão de notificação:", err);
      }
    };
    ensureNotificationPermission();

    // Register Android Back Button listener via Tauri API
    let backListener: any = null;
    onBackButtonPress(() => {
      handleBackAction();
    })
      .then((listener) => {
        backListener = listener;
      })
      .catch((err) => {
        console.log("onBackButtonPress não disponível neste ambiente:", err);
      });

    // Register Desktop Escape key listener
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        const state = navigationStateRef.current;
        if (
          state.isNoteModalOpen ||
          state.isTaskModalOpen ||
          state.showRestoreModal ||
          (!state.isSidebarCollapsed && window.innerWidth <= 768)
        ) {
          handleBackAction();
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      unlistenSuccess.then((f) => f());
      unlistenError.then((f) => f());
      unlistenDeepLink.then((f) => f());
      unlistenCloudSynced.then((f) => f());
      unlistenCloudRestored.then((f) => f());
      if (backListener && typeof backListener.unregister === "function") {
        backListener.unregister();
      }
      window.removeEventListener("keydown", handleKeyDown);
      if (toastTimeoutRef.current) clearTimeout(toastTimeoutRef.current);
      if (autoSyncTimerRef.current) clearTimeout(autoSyncTimerRef.current);
    };
  }, []);

  // Unified back navigation action (Android Back Button & Desktop Escape)
  const handleBackAction = () => {
    const state = navigationStateRef.current;

    // Priority 1: Close active modals
    if (state.isNoteModalOpen) {
      setIsNoteModalOpen(false);
      return;
    }
    if (state.isTaskModalOpen) {
      setIsTaskModalOpen(false);
      return;
    }
    if (state.showRestoreModal) {
      setShowRestoreModal(false);
      return;
    }

    // Priority 2: Close mobile sidebar drawer if open
    if (!state.isSidebarCollapsed && typeof window !== "undefined" && window.innerWidth <= 768) {
      handleToggleSidebar(true);
      return;
    }

    // Priority 3: Return to home tab ("tasks") if in a secondary tab
    if (state.activeTab !== "tasks") {
      setActiveTab("tasks");
      return;
    }

    // Priority 4: Root screen ("tasks") - Double back to exit (Android standard)
    const now = Date.now();
    if (now - lastBackPressTimeRef.current < 2000) {
      invoke("exit_app").catch((e) => console.error("Erro ao fechar aplicativo:", e));
    } else {
      lastBackPressTimeRef.current = now;
      setExitToastVisible(true);
      if (toastTimeoutRef.current) clearTimeout(toastTimeoutRef.current);
      toastTimeoutRef.current = setTimeout(() => {
        setExitToastVisible(false);
      }, 2000);
    }
  };

  const loadTasks = async () => {
    try {
      const res = await invoke<Task[]>("get_tasks");
      setTasks(res);
    } catch (e) {
      console.error("Erro ao buscar tarefas:", e);
    }
  };

  const loadNotes = async () => {
    try {
      const res = await invoke<Note[]>("get_notes");
      setNotes(res);
    } catch (e) {
      console.error("Erro ao buscar notas:", e);
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
    } catch (e) {
      console.error("Erro ao checar backups na nuvem:", e);
    }
  };

  const handleRestoreBackup = async (provider: string) => {
    setIsRestoring(true);
    try {
      await invoke("restore_backup", { provider });
      alert("Dados sincronizados com sucesso a partir da nuvem!");
      setShowRestoreModal(false);
      await loadTasks();
      await loadNotes();
      await loadSettings();
    } catch (e) {
      alert(`Erro ao restaurar: ${e}`);
    } finally {
      setIsRestoring(false);
    }
  };

  const handleRestoreSafetyBackup = async (provider: string) => {
    const confirmed = confirm(
      `Tem certeza que deseja restaurar a CÓPIA DE SEGURANÇA (24h) do ${provider === "gdrive" ? "Google Drive" : "OneDrive"}?\n\nIsso recuperará as tarefas e notas do snapshot do dia anterior.`
    );
    if (!confirmed) return;

    setIsRestoring(true);
    try {
      await invoke("restore_safety_backup", { provider });
      alert("Cópia de segurança de 24 horas restaurada com sucesso!");
      await loadTasks();
      await loadNotes();
      await loadSettings();
    } catch (e) {
      alert(`Erro ao restaurar cópia de segurança de 24h: ${e}`);
    } finally {
      setIsRestoring(false);
    }
  };

  const handleManualBackup = async () => {
    setIsBackingUp(true);
    try {
      const res = await invoke<SyncResult>("auto_sync");
      setBackupReport(res);
      await loadTasks();
      await loadNotes();
      await loadSettings();
      alert(res.message || "Sincronização concluída com sucesso!");
    } catch (e) {
      alert(`Erro na sincronização: ${e}`);
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
      triggerDebouncedSync();
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
      triggerDebouncedSync();
    } catch (e) {
      alert(`Erro ao excluir tarefa: ${e}`);
    }
  };

  // Open modal for note creation
  const openCreateNoteModal = () => {
    setEditingNote(null);
    setIsNoteModalOpen(true);
  };

  // Open modal for note editing
  const openEditNoteModal = (note: Note) => {
    setEditingNote(note);
    setIsNoteModalOpen(true);
  };

  // Save or Update note
  const handleSaveNote = async (notePayload: Note) => {
    try {
      if (editingNote) {
        await invoke("update_note", { note: notePayload });
      } else {
        await invoke("create_note", { note: notePayload });
      }
      setIsNoteModalOpen(false);
      loadNotes();
      triggerDebouncedSync();
    } catch (e) {
      alert(`Erro ao salvar nota: ${e}`);
    }
  };

  // Delete note
  const handleDeleteNote = async (id: number) => {
    try {
      await invoke("delete_note", { id });
      loadNotes();
      triggerDebouncedSync();
    } catch (e) {
      alert(`Erro ao excluir nota: ${e}`);
    }
  };

  // Toggle pin status of note
  const handleTogglePinNote = async (note: Note) => {
    try {
      const updated: Note = {
        ...note,
        is_pinned: !note.is_pinned,
        updated_at: new Date().toISOString(),
      };
      await invoke("update_note", { note: updated });
      loadNotes();
      triggerDebouncedSync();
    } catch (e) {
      console.error("Erro ao alternar fixação da nota:", e);
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
      triggerDebouncedSync();
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
      triggerDebouncedSync();
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
      triggerDebouncedSync();
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
  const handleConnectProvider = async (provider: "gdrive" | "onedrive") => {
    try {
      await invoke("start_oauth", { provider });
    } catch (e) {
      alert(`Erro ao iniciar fluxo OAuth: ${e}`);
    }
  };

  // Disconnect Cloud Provider
  const handleDisconnectProvider = async (provider: "gdrive" | "onedrive") => {
    const providerName = provider === "gdrive" ? "Google Drive" : "OneDrive";
    if (!confirm(`Deseja realmente desconectar a conta do ${providerName}?`)) {
      return;
    }
    try {
      await invoke("disconnect_provider", { provider });
      await loadSettings();
      alert(`Conta do ${providerName} desconectada com sucesso.`);
    } catch (e) {
      alert(`Erro ao desconectar ${providerName}: ${e}`);
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

      {/* Backdrop overlay for mobile drawer */}
      {!isSidebarCollapsed && (
        <div
          className="sidebar-backdrop"
          onClick={() => handleToggleSidebar(true)}
          title="Fechar Menu"
        />
      )}

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

        {activeTab === "notes" && (
          <NotesFeed
            notes={notes}
            onOpenCreateModal={openCreateNoteModal}
            onEdit={openEditNoteModal}
            onDelete={handleDeleteNote}
            onTogglePin={handleTogglePinNote}
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
            onDisconnectProvider={handleDisconnectProvider}
            onRestoreBackup={handleRestoreBackup}
            onRestoreSafetyBackup={handleRestoreSafetyBackup}
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

      {/* Note Creation & Editing Modal */}
      <NoteModal
        isOpen={isNoteModalOpen}
        onClose={() => setIsNoteModalOpen(false)}
        editingNote={editingNote}
        onSave={handleSaveNote}
        onDelete={handleDeleteNote}
      />

      {/* Startup Cloud Restore Alert Modal */}
      <RestoreModal
        isOpen={showRestoreModal}
        onClose={() => setShowRestoreModal(false)}
        backupCheck={backupCheck}
        onRestore={handleRestoreBackup}
        isRestoring={isRestoring}
      />

      {/* Android Double Back Exit Toast Notification */}
      {exitToastVisible && (
        <div className="android-exit-toast">
          Pressione voltar novamente para sair
        </div>
      )}
    </div>
  );
}

export default App;
