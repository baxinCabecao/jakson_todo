import React, { useMemo } from "react";
import { CheckCircle2, Clock, AlertTriangle, ListTodo } from "lucide-react";
import { Task } from "../types";
import "./Dashboard.css";

interface DashboardProps {
  tasks: Task[];
  getDueStatus: (dueDate?: string) => string;
}

export const Dashboard: React.FC<DashboardProps> = ({ tasks, getDueStatus }) => {
  const stats = useMemo(() => {
    let completed = 0;
    let inProgress = 0;
    let todo = 0;
    let overdue = 0;
    let dueToday = 0;

    tasks.forEach((t) => {
      if (t.status === "completed") {
        completed++;
      } else {
        if (t.status === "in_progress") inProgress++;
        if (t.status === "todo") todo++;

        const status = getDueStatus(t.due_date);
        if (status === "overdue") overdue++;
        if (status === "today") dueToday++;
      }
    });

    return { completed, inProgress, todo, overdue, dueToday, total: tasks.length };
  }, [tasks, getDueStatus]);

  const progressPercent = stats.total
    ? Math.round((stats.completed / stats.total) * 100)
    : 0;

  return (
    <div className="tab-pane animate-fade-in">
      <header className="content-header">
        <div>
          <h1>Dashboard de Desempenho</h1>
          <p>Métricas gerais e distribuição de suas tarefas pendentes e concluídas.</p>
        </div>
      </header>

      {/* Stats Cards */}
      <div className="dashboard-grid">
        <div className="stat-card">
          <h3>Total de Tarefas</h3>
          <p className="stat-number">{stats.total}</p>
          <ListTodo className="stat-icon" size={24} />
        </div>
        <div className="stat-card urgent">
          <h3>Atrasadas</h3>
          <p className="stat-number">{stats.overdue}</p>
          <AlertTriangle className="stat-icon" size={24} />
        </div>
        <div className="stat-card warning">
          <h3>Vencem Hoje</h3>
          <p className="stat-number">{stats.dueToday}</p>
          <Clock className="stat-icon" size={24} />
        </div>
        <div className="stat-card success">
          <h3>Concluídas</h3>
          <p className="stat-number">{stats.completed}</p>
          <CheckCircle2 className="stat-icon" size={24} />
        </div>
      </div>

      {/* Progress Chart Card */}
      <div className="dashboard-charts-placeholder">
        <h3>Distribuição Macro</h3>
        <div className="progress-bar-container">
          <div className="progress-bar" style={{ width: `${progressPercent}%` }}>
            {progressPercent > 12 ? `${progressPercent}% Concluído` : ""}
          </div>
        </div>

        <div className="distribution-details">
          <p>
            <strong>A Fazer:</strong> {stats.todo}
          </p>
          <p>
            <strong>Em Andamento:</strong> {stats.inProgress}
          </p>
          <p>
            <strong>Apenas pendentes:</strong> {stats.total - stats.completed}
          </p>
        </div>
      </div>
    </div>
  );
};
