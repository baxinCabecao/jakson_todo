# Jakson Todo

An elegant, portable desktop task manager built with **Tauri (v2)**, **Rust**, **React**, **TypeScript**, and **SQLite**. It bundles all resources into a single portable binary that runs without installation.

> [!NOTE]
> **AI-Driven Development:** This project was built and refined by AI coding agents in just a few hours. Despite the rapid turnaround, it strictly adheres to modern software engineering best practices, including component isolation, React Portals for layout overlay layering, solid database migrations, and clean separation of concerns.

---

## ✨ Key Features

- **Task & Subtask CRUD:** Rich task creation with custom titles, descriptions, status, date mask validation, and priority categorization.
- **Eisenhower Matrix View:** Interactive grid classifying tasks into quadrants based on urgency and importance, featuring inline calendar date updates.
- **Minimize to System Tray:** Closing the window minimizes the app to the system tray, allowing it to run silently in the background.
- **Cloud Backup & Restore:** Simultaneous manual and automatic background backups to **Google Drive** and **OneDrive** (via OAuth local TCP listener). Handles automatic restore prompt when newer cloud backups are found.
- **System Notifications:** Low-level tray notifications for backup updates, errors, and task due reminders.

---

## 🛠️ Architecture Decisions

The codebase is organized with a strict separation between UI presentation and low-level system services:

- **Backend (Rust):**
  - `db.rs`: Manages schema creation, migrations, and CRUD operations using `rusqlite` with bundled compilation.
  - `backup.rs`: Handles OAuth 2.0 refresh flow, multi-cloud uploads/downloads, and local port `59135` listener.
  - `systray.rs`: Handles native OS tray commands, click interception, and context menus.
  - `lib.rs`: Orchestrates Tauri event loops, close-to-hide window events, and background async loops.
- **Frontend (React + TS):**
  - **Component Isolation:** Every visual component (e.g., `TaskCard`, `DatePicker`, `TaskModal`, `Dashboard`) is fully isolated in its own file with its own localized CSS variables.
  - **Portal Overlay Rendering:** Dialogs and date picker dropdowns render outside the parent DOM tree to prevent layout clipping by `overflow: hidden` properties.
  - **Design System:** Sleek, responsive dark-mode styling using vanilla CSS custom properties (no Tailwind dependency).

---

## 🚀 Setup & Build Commands

### Prerequisites
- Node.js (v18+) & npm
- Rust & Cargo (`rustup`)
- Linux system dependencies (if building on Linux): `webkit2gtk-4.1-dev`, `libsoup-3.0-dev`, `libjavascriptcoregtk-4.1-dev`

### Development
```bash
npm install
npm run tauri dev
```

### Build Portable Release
```bash
npm run tauri build
```
*(Produces a release executable with embedded SQLite and client assets in `src-tauri/target/release/bundle/`)*

### Backend Tests
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## 📄 License

This project is licensed under the **GNU General Public License v3.0 (GPLv3)**. See the [LICENSE](LICENSE) file for details.
