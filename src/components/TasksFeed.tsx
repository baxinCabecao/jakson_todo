import React from "react";
import { ListTodo } from "lucide-react";
import { Task } from "../types";
import { TaskCard } from "./TaskCard";
import "./TasksFeed.css";

interface TasksFeedProps {
  tasks: Task[];
  columns: number;
  getDueStatus: (dueDate?: string) => string;
  onToggleCompleted: (task: Task) => void;
  onToggleSubtask: (task: Task, subtaskIndex: number) => void;
  onEdit: (task: Task) => void;
  onDelete: (id: number) => void;
  onUpdateDueDate: (task: Task, newDueDate: string | undefined) => void;
}

export const TasksFeed: React.FC<TasksFeedProps> = ({
  tasks,
  columns,
  getDueStatus,
  onToggleCompleted,
  onToggleSubtask,
  onEdit,
  onDelete,
  onUpdateDueDate,
}) => {
  if (tasks.length === 0) {
    return (
      <div className="empty-state">
        <ListTodo size={48} className="empty-icon" />
        <p>Nenhuma tarefa encontrada correspondendo aos filtros selecionados.</p>
      </div>
    );
  }

  return (
    <div className="tasks-feed">
      {Array.from({ length: columns }).map((_, colIdx) => (
        <div key={colIdx} className="tasks-column">
          {tasks
            .filter((_, idx) => idx % columns === colIdx)
            .map((task) => (
              <TaskCard
                key={task.id}
                task={task}
                getDueStatus={getDueStatus}
                onToggleCompleted={onToggleCompleted}
                onToggleSubtask={onToggleSubtask}
                onEdit={onEdit}
                onDelete={onDelete}
                onUpdateDueDate={onUpdateDueDate}
              />
            ))}
        </div>
      ))}
    </div>
  );
};
