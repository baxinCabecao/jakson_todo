import React from "react";
import { SlidersHorizontal, Calendar, Columns2, Columns3 } from "lucide-react";
import "./FiltersBar.css";

interface FiltersBarProps {
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  filterStatus: string;
  setFilterStatus: (status: string) => void;
  filterPriority: string;
  setFilterPriority: (priority: string) => void;
  filterDueDate: string;
  setFilterDueDate: (dueDate: string) => void;
  sortBy: "due_date" | "priority" | "title";
  setSortBy: (sortBy: "due_date" | "priority" | "title") => void;
  showCompleted: boolean;
  setShowCompleted: (showCompleted: boolean) => void;
  columns: 2 | 3;
  setColumns: (columns: 2 | 3) => void;
}

export const FiltersBar: React.FC<FiltersBarProps> = ({
  searchQuery,
  setSearchQuery,
  filterStatus,
  setFilterStatus,
  filterPriority,
  setFilterPriority,
  filterDueDate,
  setFilterDueDate,
  sortBy,
  setSortBy,
  showCompleted,
  setShowCompleted,
  columns,
  setColumns,
}) => {
  return (
    <section className="filters-bar-card">
      <div className="search-box">
        <input
          type="text"
          placeholder="Pesquisar tarefas pelo título ou descrição..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
      </div>

      <div className="filters-row">
        <div className="filter-group">
          <SlidersHorizontal size={14} className="filter-icon" />
          <select value={filterStatus} onChange={(e) => setFilterStatus(e.target.value)}>
            <option value="all">Todos os Status</option>
            <option value="todo">A Fazer</option>
            <option value="in_progress">Em Andamento</option>
            <option value="completed">Concluídas</option>
          </select>
        </div>

        <div className="filter-group">
          <select value={filterPriority} onChange={(e) => setFilterPriority(e.target.value)}>
            <option value="all">Todas as Prioridades</option>
            <option value="high">Prioridade Alta</option>
            <option value="medium">Prioridade Média</option>
            <option value="low">Prioridade Baixa</option>
          </select>
        </div>

        <div className="filter-group">
          <Calendar size={14} className="filter-icon" />
          <select value={filterDueDate} onChange={(e) => setFilterDueDate(e.target.value)}>
            <option value="all">Qualquer Vencimento</option>
            <option value="today">Vence Hoje</option>
            <option value="tomorrow">Vence Amanhã</option>
            <option value="overdue">Atrasadas</option>
          </select>
        </div>

        <div className="filter-group">
          <select value={sortBy} onChange={(e) => setSortBy(e.target.value as any)}>
            <option value="due_date">Ordenar por Vencimento</option>
            <option value="priority">Ordenar por Prioridade</option>
            <option value="title">Ordenar por Nome</option>
          </select>
        </div>

        <label className="toggle-completed-label">
          <input
            type="checkbox"
            checked={showCompleted}
            onChange={(e) => setShowCompleted(e.target.checked)}
          />
          <span>Mostrar Concluídas</span>
        </label>

        <div className="layout-selector">
          <button
            type="button"
            className={`btn-layout ${columns === 2 ? "active" : ""}`}
            onClick={() => setColumns(2)}
            title="Visualização em 2 colunas"
          >
            <Columns2 size={16} />
          </button>
          <button
            type="button"
            className={`btn-layout ${columns === 3 ? "active" : ""}`}
            onClick={() => setColumns(3)}
            title="Visualização em 3 colunas"
          >
            <Columns3 size={16} />
          </button>
        </div>
      </div>
    </section>
  );
};
