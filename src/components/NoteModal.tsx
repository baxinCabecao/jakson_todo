import React, { useState, useEffect, useRef } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  X,
  Pin,
  Upload,
  Eye,
  Edit3,
  Columns,
  Bold,
  Italic,
  Heading,
  List,
  ListOrdered,
  CheckSquare,
  Code,
  Quote,
  Link,
  Trash2,
  Wand2,
} from "lucide-react";
import { Note } from "../types";
import "./NoteModal.css";

interface NoteModalProps {
  isOpen: boolean;
  onClose: () => void;
  editingNote: Note | null;
  onSave: (note: Note) => void;
  onDelete?: (id: number) => void;
}

const FORMAT_BUTTONS = [
  { label: "Negrito", icon: Bold, prefix: "**", suffix: "**", placeholder: "texto" },
  { label: "Itálico", icon: Italic, prefix: "*", suffix: "*", placeholder: "texto" },
  { label: "Título", icon: Heading, prefix: "# ", suffix: "", placeholder: "Título" },
  { divider: true },
  { label: "Lista", icon: List, prefix: "- ", suffix: "", placeholder: "Item" },
  { label: "Numerada", icon: ListOrdered, prefix: "1. ", suffix: "", placeholder: "Item" },
  { label: "Tarefa", icon: CheckSquare, prefix: "- [ ] ", suffix: "", placeholder: "Tarefa" },
  { divider: true },
  { label: "Código", icon: Code, prefix: "`", suffix: "`", placeholder: "código" },
  { label: "Citação", icon: Quote, prefix: "> ", suffix: "", placeholder: "Citação" },
  { label: "Link", icon: Link, prefix: "[", suffix: "](https://)", placeholder: "link" },
];

export const NoteModal: React.FC<NoteModalProps> = ({
  isOpen,
  onClose,
  editingNote,
  onSave,
  onDelete,
}) => {
  const [title, setTitle] = useState("");
  const [isTitleManual, setIsTitleManual] = useState(false);
  const [content, setContent] = useState("");
  const [isPinned, setIsPinned] = useState(false);
  const [viewMode, setViewMode] = useState<"edit" | "preview" | "split">("edit");
  const [showShortcuts, setShowShortcuts] = useState<boolean>(() => {
    const saved = localStorage.getItem("todo-note-show-shortcuts");
    if (saved !== null) return saved === "true";
    return typeof window !== "undefined" && window.innerWidth > 768;
  });

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Initialize or reset form state
  useEffect(() => {
    if (editingNote) {
      setTitle(editingNote.title);
      setContent(editingNote.content || "");
      setIsPinned(editingNote.is_pinned);
      setIsTitleManual(true);
    } else {
      setTitle("");
      setContent("");
      setIsPinned(false);
      setIsTitleManual(false);
    }
    if (typeof window !== "undefined" && window.innerWidth > 768) {
      setViewMode("split");
    } else {
      setViewMode("edit");
    }
  }, [editingNote, isOpen]);

  if (!isOpen) return null;

  // Extract heading (# Title) from content
  const extractHeading = (text: string): string => {
    const lines = text.split("\n");
    for (const rawLine of lines) {
      const line = rawLine.trim();
      const match = line.match(/^#{1,6}\s+(.+)$/);
      if (match && match[1].trim()) {
        return match[1].trim();
      }
    }
    return "";
  };

  const handleTitleChange = (newTitle: string) => {
    setTitle(newTitle);
    setIsTitleManual(true);
  };

  const handleContentChange = (newContent: string) => {
    setContent(newContent);
    // If user hasn't explicitly typed in the title field, extract # Heading dynamically
    if (!isTitleManual) {
      const detected = extractHeading(newContent);
      if (detected) {
        setTitle(detected);
      }
    }
  };

  const toggleShortcuts = () => {
    setShowShortcuts((prev) => {
      const next = !prev;
      localStorage.setItem("todo-note-show-shortcuts", String(next));
      return next;
    });
  };

  const insertFormatting = (prefix: string, suffix: string = "", placeholder: string = "") => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const selectedText = content.substring(start, end) || placeholder;
    const replacement = `${prefix}${selectedText}${suffix}`;

    const newContent = content.substring(0, start) + replacement + content.substring(end);
    handleContentChange(newContent);

    setTimeout(() => {
      textarea.focus();
      const newCursor = start + prefix.length + selectedText.length;
      textarea.setSelectionRange(newCursor, newCursor);
    }, 10);
  };

  const handleFileImport = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      const text = (event.target?.result as string) || "";
      setContent(text);

      const heading = extractHeading(text);
      if (heading) {
        setTitle(heading);
      } else {
        const nameWithoutExt = file.name.replace(/\.[^/.]+$/, "");
        setTitle(nameWithoutExt);
      }
      setIsTitleManual(true);
    };
    reader.readAsText(file);
    e.target.value = "";
  };

  const handleSubmit = (e?: React.FormEvent) => {
    if (e) e.preventDefault();

    let finalTitle = title.trim();
    if (!finalTitle) {
      finalTitle = extractHeading(content);
    }
    if (!finalTitle) {
      const firstLine = content
        .split("\n")
        .map((l) => l.replace(/^[>\-*_`~#\s]+/, "").trim())
        .find((l) => l.length > 0);
      if (firstLine) {
        finalTitle = firstLine.length > 40 ? firstLine.substring(0, 40) + "..." : firstLine;
      }
    }

    if (!finalTitle) {
      alert(
        "O título é obrigatório. Por favor, digite um título no campo acima ou inicie o texto com um cabeçalho (# Título)."
      );
      return;
    }

    const now = new Date().toISOString();
    const notePayload: Note = {
      id: editingNote?.id,
      title: finalTitle,
      content,
      is_pinned: isPinned,
      created_at: editingNote ? editingNote.created_at : now,
      updated_at: now,
    };

    onSave(notePayload);
  };

  const handleDelete = () => {
    if (editingNote?.id && onDelete) {
      if (confirm("Tem certeza que deseja excluir esta nota permanentemente?")) {
        onDelete(editingNote.id);
        onClose();
      }
    }
  };

  return (
    <div className="modal-backdrop">
      <div className="note-modal-card animate-fade-in">
        {/* Header */}
        <header className="note-modal-header">
          <div className="note-title-container">
            <input
              type="text"
              className="note-title-input"
              placeholder="Título da nota (ou use # Título no texto)..."
              value={title}
              onChange={(e) => handleTitleChange(e.target.value)}
              autoFocus
            />
          </div>

          <div className="note-header-actions">
            <button
              type="button"
              className={`btn-note-header ${isPinned ? "active" : ""}`}
              onClick={() => setIsPinned(!isPinned)}
              title={isPinned ? "Desafixar nota" : "Fixar nota no topo"}
            >
              <Pin size={16} fill={isPinned ? "currentColor" : "none"} />
              <span>{isPinned ? "Fixada" : "Fixar"}</span>
            </button>

            <button
              type="button"
              className="btn-note-header"
              onClick={() => fileInputRef.current?.click()}
              title="Importar arquivo Markdown ou Texto (.md, .txt)"
            >
              <Upload size={16} />
              <span>Importar</span>
            </button>
            <input
              type="file"
              ref={fileInputRef}
              accept=".md,.txt,.markdown"
              style={{ display: "none" }}
              onChange={handleFileImport}
            />

            {editingNote && onDelete && (
              <button
                type="button"
                className="btn-note-header danger"
                onClick={handleDelete}
                title="Excluir nota"
              >
                <Trash2 size={16} />
              </button>
            )}

            <button type="button" className="btn-close" onClick={onClose} title="Fechar">
              <X size={20} />
            </button>
          </div>
        </header>

        {/* Toolbar Controls: View Modes (Always Visible) & Shortcuts Toggle */}
        <div className="note-modal-toolbar-row">
          <div className="note-toolbar-controls">
            <div className="note-view-modes">
              <button
                type="button"
                className={`btn-view-mode ${viewMode === "edit" ? "active" : ""}`}
                onClick={() => setViewMode("edit")}
                title="Modo Edição (Raw Markdown)"
              >
                <Edit3 size={14} />
                <span>Editar</span>
              </button>
              <button
                type="button"
                className={`btn-view-mode ${viewMode === "preview" ? "active" : ""}`}
                onClick={() => setViewMode("preview")}
                title="Modo Visualização (Renderizado)"
              >
                <Eye size={14} />
                <span>Visualizar</span>
              </button>
              <button
                type="button"
                className={`btn-view-mode btn-split-mode ${viewMode === "split" ? "active" : ""}`}
                onClick={() => setViewMode("split")}
                title="Modo Lado a Lado (Split View)"
              >
                <Columns size={14} />
                <span>Lado a Lado</span>
              </button>
            </div>

            <button
              type="button"
              className={`btn-toggle-shortcuts ${showShortcuts ? "active" : ""}`}
              onClick={toggleShortcuts}
              title={showShortcuts ? "Ocultar menu de atalhos" : "Mostrar menu de atalhos"}
            >
              <Wand2 size={14} />
              <span>{showShortcuts ? "Ocultar Atalhos" : "Atalhos"}</span>
            </button>
          </div>

          {/* Formatting Shortcuts Toolbar */}
          {showShortcuts && viewMode !== "preview" && (
            <div className="note-formatting-toolbar animate-fade-in">
              {FORMAT_BUTTONS.map((btn, index) =>
                btn.divider ? (
                  <div key={`divider-${index}`} className="toolbar-divider" />
                ) : (
                  <button
                    key={btn.label}
                    type="button"
                    className="btn-format"
                    onClick={() => insertFormatting(btn.prefix!, btn.suffix, btn.placeholder)}
                    title={btn.label}
                  >
                    {btn.icon && <btn.icon size={15} />}
                  </button>
                )
              )}
            </div>
          )}
        </div>

        {/* Editor Body Area */}
        <div className="note-modal-body">
          {(viewMode === "edit" || viewMode === "split") && (
            <div className="note-editor-pane">
              <textarea
                ref={textareaRef}
                className="note-textarea"
                placeholder="Escreva sua nota em Markdown... (Dica: use # Título para definir o título automaticamente)"
                value={content}
                onChange={(e) => handleContentChange(e.target.value)}
              />
            </div>
          )}

          {(viewMode === "preview" || viewMode === "split") && (
            <div className="note-preview-pane">
              <div className="note-preview-scroll">
                {content.trim() ? (
                  <div className="markdown-preview">
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>
                      {content}
                    </ReactMarkdown>
                  </div>
                ) : (
                  <p style={{ color: "var(--text-muted)", fontStyle: "italic" }}>
                    Nenhum conteúdo para visualizar...
                  </p>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <footer className="note-modal-footer">
          <div className="note-footer-info">
            <span>{content.length} caracteres</span>
          </div>

          <div className="note-footer-actions">
            <button type="button" className="btn-secondary" onClick={onClose}>
              Cancelar
            </button>
            <button
              type="button"
              className="btn-primary"
              onClick={() => handleSubmit()}
            >
              Salvar Nota
            </button>
          </div>
        </footer>
      </div>
    </div>
  );
};
