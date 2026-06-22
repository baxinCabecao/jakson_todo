import React, { useMemo } from "react";
import { AlertTriangle, Calendar, Clock, Trash2, Edit3, CheckCircle2, Square, Plus } from "lucide-react";
import { Task } from "../types";
import { DatePicker } from "./DatePicker";
import "./EisenhowerMatrix.css";

interface EisenhowerMatrixProps {
  tasks: Task[];
  getDueStatus: (dueDate?: string) => string;
  onEdit: (task: Task) => void;
  onToggleCompleted: (task: Task) => void;
  onOpenCreateModal: () => void;
  showCompleted: boolean;
  setShowCompleted: (val: boolean) => void;
  onUpdateDueDate: (task: Task, newDueDate: string | undefined) => void;
}

const toLocalFormat = (dbDate?: string): string => {
  if (!dbDate) return "";
  const parts = dbDate.split("-");
  if (parts.length === 3) {
    const [year, month, day] = parts;
    return `${day}/${month}/${year}`;
  }
  return dbDate;
};

export const EisenhowerMatrix: React.FC<EisenhowerMatrixProps> = ({
  tasks,
  getDueStatus,
  onEdit,
  onToggleCompleted,
  onOpenCreateModal,
  showCompleted,
  setShowCompleted,
  onUpdateDueDate,
}) => {

  // Categorize tasks into the four Eisenhower quadrants
  const quadrants = useMemo(() => {
    const q1: Task[] = []; // Urgent & Important (High/Medium Priority + Overdue/Today/Tomorrow)
    const q2: Task[] = []; // Not Urgent & Important (High/Medium Priority + Future/No Due Date)
    const q3: Task[] = []; // Urgent & Not Important (Low Priority + Overdue/Today/Tomorrow)
    const q4: Task[] = []; // Not Urgent & Not Important (Low Priority + Future/No Due Date)

    tasks.forEach((task) => {
      // Filter out completed tasks if showCompleted is false
      if (!showCompleted && task.status === "completed") {
        return;
      }

      const status = getDueStatus(task.due_date);
      const isUrgent = status === "overdue" || status === "today" || status === "tomorrow";
      const isImportant = task.priority === "high" || task.priority === "medium";

      if (isImportant && isUrgent) {
        q1.push(task);
      } else if (isImportant && !isUrgent) {
        q2.push(task);
      } else if (!isImportant && isUrgent) {
        q3.push(task);
      } else {
        q4.push(task);
      }
    });

    const sortByDueDate = (a: Task, b: Task) => {
      if (!a.due_date && b.due_date) return 1;
      if (a.due_date && !b.due_date) return -1;
      if (a.due_date && b.due_date) {
        return a.due_date.localeCompare(b.due_date);
      }
      return 0;
    };

    q1.sort(sortByDueDate);
    q2.sort(sortByDueDate);
    q3.sort(sortByDueDate);
    q4.sort(sortByDueDate);

    return { q1, q2, q3, q4 };
  }, [tasks, getDueStatus, showCompleted]);

  const renderTaskItem = (task: Task) => {
    const dueStatus = getDueStatus(task.due_date);
    const isCompleted = task.status === "completed";

    return (
      <div key={task.id} className={`eisenhower-item priority-${task.priority} ${isCompleted ? "completed" : ""}`}>
        <button
          type="button"
          className="eisenhower-check-btn"
          onClick={() => onToggleCompleted(task)}
        >
          {isCompleted ? (
            <CheckCircle2 size={16} className="checked-icon" />
          ) : (
            <Square size={16} className="unchecked-icon" />
          )}
        </button>
        
        <span className="eisenhower-item-title" onClick={() => onEdit(task)}>
          {task.title}
        </span>

        <div className="eisenhower-item-meta">
          <DatePicker
            value={task.due_date}
            onChange={(newDate) => onUpdateDueDate(task, newDate)}
          >
            {task.due_date ? (
              <span className={`eisenhower-date-badge ${dueStatus}`} title="Alterar Vencimento">
                {toLocalFormat(task.due_date)}
              </span>
            ) : (
              <span className="eisenhower-date-badge no-due" title="Definir Vencimento">
                + Vencimento
              </span>
            )}
          </DatePicker>
          <button
            type="button"
            className="eisenhower-edit-btn"
            onClick={() => onEdit(task)}
            title="Editar Tarefa"
          >
            <Edit3 size={12} />
          </button>
        </div>
      </div>
    );
  };

  return (
    <div className="tab-pane animate-fade-in">
      <header className="content-header">
        <div>
          <h1>Matriz de Eisenhower</h1>
          <p>Priorize suas tarefas de acordo com urgência e importância.</p>
        </div>
        <div className="matrix-header-actions">
          <label className="toggle-completed-label">
            <input
              type="checkbox"
              checked={showCompleted}
              onChange={(e) => setShowCompleted(e.target.checked)}
            />
            <span>Mostrar Concluídas</span>
          </label>
          <button className="btn-primary" onClick={onOpenCreateModal}>
            <Plus size={18} />
            <span>Nova Tarefa</span>
          </button>
        </div>
      </header>

      <div className="eisenhower-matrix-container">
        {/* Y-Axis Labels Container */}
        <div className="matrix-y-axis-container">
          <div className="text-important">Importante</div>
          <div className="text-not-important">Não Importante</div>
        </div>

        {/* X-Axis Labels Container */}
        <div className="matrix-x-axis-container">
          <div className="text-urgent">Urgente</div>
          <div className="text-not-urgent">Não Urgente</div>
        </div>

        {/* The 2x2 Grid */}
        <div className="eisenhower-grid">
          {/* Q1: Urgent & Important */}
          <div className="matrix-quadrant q1-urgent-important">
            <div className="quadrant-header">
              <AlertTriangle className="quadrant-icon" size={18} />
              <div>
                <h3>1. Fazer Agora</h3>
                <span className="quadrant-subtitle">Urgente e Importante</span>
              </div>
              <span className="quadrant-count">{quadrants.q1.length}</span>
            </div>
            <div className="quadrant-list">
              {quadrants.q1.length === 0 ? (
                <div className="quadrant-empty">Nenhuma tarefa urgente e importante</div>
              ) : (
                quadrants.q1.map(renderTaskItem)
              )}
            </div>
          </div>

          {/* Q2: Not Urgent & Important */}
          <div className="matrix-quadrant q2-important-not-urgent">
            <div className="quadrant-header">
              <Calendar className="quadrant-icon" size={18} />
              <div>
                <h3>2. Agendar</h3>
                <span className="quadrant-subtitle">Não Urgente e Importante</span>
              </div>
              <span className="quadrant-count">{quadrants.q2.length}</span>
            </div>
            <div className="quadrant-list">
              {quadrants.q2.length === 0 ? (
                <div className="quadrant-empty">Nenhuma tarefa importante de longo prazo</div>
              ) : (
                quadrants.q2.map(renderTaskItem)
              )}
            </div>
          </div>

          {/* Q3: Urgent & Not Important */}
          <div className="matrix-quadrant q3-urgent-not-important">
            <div className="quadrant-header">
              <Clock className="quadrant-icon" size={18} />
              <div>
                <h3>3. Delegar</h3>
                <span className="quadrant-subtitle">Urgente e Não Importante</span>
              </div>
              <span className="quadrant-count">{quadrants.q3.length}</span>
            </div>
            <div className="quadrant-list">
              {quadrants.q3.length === 0 ? (
                <div className="quadrant-empty">Nenhuma tarefa urgente secundária</div>
              ) : (
                quadrants.q3.map(renderTaskItem)
              )}
            </div>
          </div>

          {/* Q4: Not Urgent & Not Important */}
          <div className="matrix-quadrant q4-not-urgent-not-important">
            <div className="quadrant-header">
              <Trash2 className="quadrant-icon" size={18} />
              <div>
                <h3>4. Eliminar</h3>
                <span className="quadrant-subtitle">Não Urgente e Não Importante</span>
              </div>
              <span className="quadrant-count">{quadrants.q4.length}</span>
            </div>
            <div className="quadrant-list">
              {quadrants.q4.length === 0 ? (
                <div className="quadrant-empty">Nenhuma tarefa supérflua</div>
              ) : (
                quadrants.q4.map(renderTaskItem)
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
