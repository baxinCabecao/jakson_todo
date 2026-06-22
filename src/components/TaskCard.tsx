import React from "react";
import { Calendar, Square, CheckCircle2, Edit3, Trash2 } from "lucide-react";
import { Task } from "../types";
import { DatePicker } from "./DatePicker";
import "./TaskCard.css";

interface TaskCardProps {
  task: Task;
  getDueStatus: (dueDate?: string) => string;
  onToggleCompleted: (task: Task) => void;
  onToggleSubtask: (task: Task, subtaskIndex: number) => void;
  onEdit: (task: Task) => void;
  onDelete: (id: number) => void;
  onUpdateDueDate: (task: Task, newDueDate: string | undefined) => void;
}

const formatDateToShow = (dateStr?: string) => {
  if (!dateStr) return "";
  const parts = dateStr.split("-");
  if (parts.length === 3) {
    const [year, month, day] = parts;
    return `${day}/${month}/${year}`;
  }
  return dateStr;
};

const getPriorityLabel = (priority: string): string => {
  const p = priority.toLowerCase();
  if (p === "high" || p === "alta") return "ALTA";
  if (p === "medium" || p === "média" || p === "media") return "MÉDIA";
  if (p === "low" || p === "baixa") return "BAIXA";
  return priority.toUpperCase();
};


export const TaskCard: React.FC<TaskCardProps> = ({
  task,
  getDueStatus,
  onToggleCompleted,
  onToggleSubtask,
  onEdit,
  onDelete,
  onUpdateDueDate,
}) => {
  const dueStatus = getDueStatus(task.due_date);

  return (
    <article
      className={`task-card ${dueStatus === "overdue" ? "card-overdue" : ""} ${
        dueStatus === "today" ? "card-due-today" : ""
      } ${task.status === "completed" ? "card-completed" : ""}`}
    >
      <div className="task-card-header">
        <button className="btn-check" onClick={() => onToggleCompleted(task)}>
          {task.status === "completed" ? (
            <CheckCircle2 className="checked-icon" size={22} />
          ) : (
            <Square className="unchecked-icon" size={22} />
          )}
        </button>

        <div className="task-header-info">
          <h3 className={task.status === "completed" ? "completed-line" : ""}>
            <button
              className="task-title-link"
              onClick={() => onEdit(task)}
              title="Editar Tarefa"
            >
              {task.title}
            </button>
          </h3>
          <div className="task-meta">
            <DatePicker
              value={task.due_date}
              onChange={(newDate) => onUpdateDueDate(task, newDate)}
            >
              {task.due_date ? (
                <span className={`meta-due ${dueStatus}`} title="Alterar Vencimento">
                  <Calendar size={12} />
                  <span>{formatDateToShow(task.due_date)}</span>
                  {dueStatus === "overdue" && (
                    <span className="due-badge overdue-badge">Atrasada</span>
                  )}
                  {dueStatus === "today" && (
                    <span className="due-badge today-badge">Hoje</span>
                  )}
                  {dueStatus === "tomorrow" && (
                    <span className="due-badge tomorrow-badge">Amanhã</span>
                  )}
                </span>
              ) : (
                <span className="meta-due no-due" title="Definir Vencimento">
                  <Calendar size={12} />
                  <span>+ Vencimento</span>
                </span>
              )}
            </DatePicker>
            <span className={`meta-priority ${task.priority}`}>
              {getPriorityLabel(task.priority)}
            </span>
            <span className={`meta-status ${task.status}`}>
              {task.status === "todo" && "A Fazer"}
              {task.status === "in_progress" && "Em Andamento"}
              {task.status === "completed" && "Concluída"}
            </span>
          </div>
        </div>

        <div className="task-actions">
          <button className="btn-icon" title="Editar" onClick={() => onEdit(task)}>
            <Edit3 size={16} />
          </button>
          <button
            className="btn-icon danger"
            title="Excluir"
            onClick={() => onDelete(task.id!)}
          >
            <Trash2 size={16} />
          </button>
        </div>
      </div>

      {task.description && <p className="task-description">{task.description}</p>}

      {/* Subtasks Section */}
      {task.subtasks && task.subtasks.length > 0 && (
        <div className="task-subtasks">
          <h4>
            Subtarefas ({task.subtasks.filter((s) => s.completed).length}/
            {task.subtasks.length})
          </h4>
          <div className="subtasks-list">
            {task.subtasks.map((sub, sIdx) => (
              <div
                key={sub.id || sIdx}
                className={`subtask-item ${sub.completed ? "completed" : ""}`}
              >
                <button
                  className="btn-subtask-check"
                  onClick={() => onToggleSubtask(task, sIdx)}
                >
                  {sub.completed ? (
                    <CheckCircle2 className="checked-icon" size={16} />
                  ) : (
                    <Square className="unchecked-icon" size={16} />
                  )}
                </button>
                <span>{sub.title}</span>
                <span className={`subtask-priority ${sub.priority}`}>
                  {getPriorityLabel(sub.priority)}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </article>
  );
};
