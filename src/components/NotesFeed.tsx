import React, { useState, useMemo } from "react";
import { Plus, Search, Pin, Trash2, FileText } from "lucide-react";
import { Note } from "../types";
import "./NotesFeed.css";

interface NotesFeedProps {
  notes: Note[];
  onOpenCreateModal: () => void;
  onEdit: (note: Note) => void;
  onDelete: (id: number) => void;
  onTogglePin: (note: Note) => void;
}

export const NotesFeed: React.FC<NotesFeedProps> = ({
  notes,
  onOpenCreateModal,
  onEdit,
  onDelete,
  onTogglePin,
}) => {
  const [searchQuery, setSearchQuery] = useState("");

  // Filter notes by search query
  const filteredNotes = useMemo(() => {
    if (!searchQuery.trim()) return notes;
    const query = searchQuery.toLowerCase();
    return notes.filter(
      (n) =>
        n.title.toLowerCase().includes(query) ||
        n.content.toLowerCase().includes(query)
    );
  }, [notes, searchQuery]);

  // Separate pinned and unpinned notes
  const pinnedNotes = useMemo(
    () => filteredNotes.filter((n) => n.is_pinned),
    [filteredNotes]
  );
  const unpinnedNotes = useMemo(
    () => filteredNotes.filter((n) => !n.is_pinned),
    [filteredNotes]
  );

  const handleDelete = (e: React.MouseEvent, id: number) => {
    e.stopPropagation();
    if (confirm("Tem certeza que deseja excluir esta nota?")) {
      onDelete(id);
    }
  };

  const handleTogglePin = (e: React.MouseEvent, note: Note) => {
    e.stopPropagation();
    onTogglePin(note);
  };

  return (
    <div className="notes-feed-container animate-fade-in">
      {/* Header */}
      <header className="content-header">
        <div>
          <h1>Bloco de Notas</h1>
          <p>Crie, edite e organize suas anotações em Markdown.</p>
        </div>
        <button className="btn-primary" onClick={onOpenCreateModal}>
          <Plus size={18} />
          <span>Nova Nota</span>
        </button>
      </header>

      {/* Search Bar */}
      <div className="notes-header-actions">
        <div className="notes-search-wrapper">
          <Search size={16} className="notes-search-icon" />
          <input
            type="text"
            className="notes-search-input"
            placeholder="Buscar nas notas..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
      </div>

      {/* Empty State */}
      {filteredNotes.length === 0 && (
        <div className="notes-empty-state">
          <FileText size={38} />
          {searchQuery.trim() ? (
            <p>Nenhuma nota corresponde à busca "{searchQuery}".</p>
          ) : (
            <>
              <p>Nenhuma nota cadastrada.</p>
              <span>Clique em "Nova Nota" para criar sua primeira anotação em Markdown.</span>
            </>
          )}
        </div>
      )}

      {/* Pinned Notes Section */}
      {pinnedNotes.length > 0 && (
        <section className="notes-section">
          <div className="notes-section-title">
            <Pin size={14} fill="currentColor" />
            <span>Notas Fixadas ({pinnedNotes.length})</span>
          </div>
          <div className="notes-grid">
            {pinnedNotes.map((note) => (
              <div
                key={note.id}
                className="note-item-card pinned"
                onClick={() => onEdit(note)}
                title="Clique para editar"
              >
                <div className="note-item-content">
                  <h4 className="note-item-title">{note.title}</h4>
                </div>

                <div className="note-item-actions">
                  <button
                    type="button"
                    className="btn-note-action pinned"
                    onClick={(e) => handleTogglePin(e, note)}
                    title="Desafixar nota"
                  >
                    <Pin size={16} fill="currentColor" />
                  </button>

                  <button
                    type="button"
                    className="btn-note-action danger"
                    onClick={(e) => handleDelete(e, note.id!)}
                    title="Excluir nota"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* Regular Notes Section */}
      {unpinnedNotes.length > 0 && (
        <section className="notes-section">
          {pinnedNotes.length > 0 && (
            <div className="notes-section-title">
              <span>Outras Notas ({unpinnedNotes.length})</span>
            </div>
          )}
          <div className="notes-grid">
            {unpinnedNotes.map((note) => (
              <div
                key={note.id}
                className="note-item-card"
                onClick={() => onEdit(note)}
                title="Clique para editar"
              >
                <div className="note-item-content">
                  <h4 className="note-item-title">{note.title}</h4>
                </div>

                <div className="note-item-actions">
                  <button
                    type="button"
                    className="btn-note-action"
                    onClick={(e) => handleTogglePin(e, note)}
                    title="Fixar no topo"
                  >
                    <Pin size={16} />
                  </button>

                  <button
                    type="button"
                    className="btn-note-action danger"
                    onClick={(e) => handleDelete(e, note.id!)}
                    title="Excluir nota"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
};
