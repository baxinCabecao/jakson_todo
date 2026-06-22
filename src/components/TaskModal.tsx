import React, { useState, useEffect } from "react";
import { X, Trash2, Square, CheckCircle2 } from "lucide-react";
import { Task, Subtask } from "../types";
import { DatePicker } from "./DatePicker";
import "./TaskModal.css";

interface TaskModalProps {
  isOpen: boolean;
  onClose: () => void;
  editingTask: Task | null;
  onSave: (task: Task) => void;
}

export const TaskModal: React.FC<TaskModalProps> = ({
  isOpen,
  onClose,
  editingTask,
  onSave,
}) => {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [dueDate, setDueDate] = useState("");
  const [priority, setPriority] = useState("medium");
  const [status, setStatus] = useState("todo");
  const [subtasks, setSubtasks] = useState<Subtask[]>([]);
  const [newSubtaskTitle, setNewSubtaskTitle] = useState("");

  // Initialize form when editingTask changes or modal opens
  useEffect(() => {
    if (editingTask) {
      setTitle(editingTask.title);
      setDescription(editingTask.description || "");
      setDueDate(editingTask.due_date || "");
      setPriority(editingTask.priority);
      setStatus(editingTask.status);
      setSubtasks(editingTask.subtasks || []);
    } else {
      setTitle("");
      setDescription("");
      setDueDate("");
      setPriority("medium");
      setStatus("todo");
      setSubtasks([]);
    }
    setNewSubtaskTitle("");
  }, [editingTask, isOpen]);

  if (!isOpen) return null;

  const handleAddSubtask = () => {
    if (newSubtaskTitle.trim() === "") return;
    const newSub: Subtask = {
      title: newSubtaskTitle.trim(),
      priority: "medium",
      completed: false,
      created_at: new Date().toISOString(),
    };
    setSubtasks([...subtasks, newSub]);
    setNewSubtaskTitle("");
  };

  const handleRemoveSubtask = (index: number) => {
    setSubtasks(subtasks.filter((_, i) => i !== index));
  };

  const handleToggleSubtask = (index: number) => {
    const updated = [...subtasks];
    updated[index].completed = !updated[index].completed;
    setSubtasks(updated);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (title.trim() === "") return;

    const taskPayload: Task = {
      id: editingTask?.id,
      title: title.trim(),
      description: description.trim() ? description : undefined,
      due_date: dueDate || undefined,
      priority,
      status,
      created_at: editingTask ? editingTask.created_at : new Date().toISOString(),
      subtasks,
    };

    onSave(taskPayload);
  };

  return (
    <div className="modal-backdrop">
      <div className="modal-card animate-fade-in">
        <div className="modal-header">
          <input
            type="text"
            required
            className="modal-title-input"
            placeholder="Informe o título da tarefa..."
            value={title}
            onChange={(e) => setTitle(e.target.value)}
          />
          <button className="btn-close" onClick={onClose} type="button">
            <X size={20} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="modal-form">
          <div className="form-group-field">
            <label>Descrição (Opcional)</label>
            <textarea
              placeholder="Insira detalhes adicionais sobre esta tarefa..."
              rows={2}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>

          <div className="form-group-row">
            <div className="form-group-field">
              <label>Data de Vencimento</label>
              <DatePicker
                value={dueDate || undefined}
                onChange={(val) => setDueDate(val || "")}
              />
            </div>

            <div className="form-group-field">
              <label>Prioridade</label>
              <select value={priority} onChange={(e) => setPriority(e.target.value)}>
                <option value="high">Alta</option>
                <option value="medium">Média</option>
                <option value="low">Baixa</option>
              </select>
            </div>

            <div className="form-group-field">
              <label>Status</label>
              <select value={status} onChange={(e) => setStatus(e.target.value)}>
                <option value="todo">A Fazer</option>
                <option value="in_progress">Em Andamento</option>
                <option value="completed">Concluída</option>
              </select>
            </div>
          </div>

          {/* Subtasks Builder */}
          <div className="subtasks-builder">
            <h3>Subtarefas</h3>
            <div className="subtasks-input-row">
              <input
                type="text"
                placeholder="Adicionar subtarefa..."
                value={newSubtaskTitle}
                onChange={(e) => setNewSubtaskTitle(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    handleAddSubtask();
                  }
                }}
              />
              <button type="button" className="btn-secondary" onClick={handleAddSubtask}>
                Adicionar
              </button>
            </div>

            <div className="subtasks-builder-list">
              {subtasks.map((sub, idx) => (
                <div key={idx} className="subtask-builder-item">
                  <button
                    type="button"
                    className="btn-subtask-check"
                    onClick={() => handleToggleSubtask(idx)}
                  >
                    {sub.completed ? (
                      <CheckCircle2 className="checked-icon" size={16} />
                    ) : (
                      <Square className="unchecked-icon" size={16} />
                    )}
                  </button>
                  <span className={sub.completed ? "completed-line" : ""}>
                    {sub.title}
                  </span>
                  <button
                    type="button"
                    className="btn-icon danger sm"
                    onClick={() => handleRemoveSubtask(idx)}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
            </div>
          </div>

          <div className="modal-footer">
            <button type="button" className="btn-secondary" onClick={onClose}>
              Cancelar
            </button>
            <button type="submit" className="btn-primary">
              {editingTask ? "Salvar Alterações" : "Criar Tarefa"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
