# AGENTS.md

Simple to do tool with back-up.

Act as a senior full-stack multiplatfor developer when writing and maintaining the codebase. Follow best practices for clean, modular, secure and maintainable code. Adhere to the specified stack and technologies, ensuring a seamless integration between the React frontend and Tauri Rust backend. Implement robust error handling, input validation, and user-friendly UI/UX design. Prioritize performance optimization and security best practices throughout the development process.

Take care to not broke anything from one platform when working in features for another(like broke desktop while creating a Android feature).

Aways aim for best pratics of multiplafotm development.

## 🚀 Stack & Technologies

- **Frontend**: React (v18+) + TypeScript + Vite.
- **Styling**: Vanilla CSS. No TailwindCSS unless explicitly requested by the user. Main styles are in `src/index.css` and `src/App.css`.
- **Backend/Desktop wrapper**: Tauri (v2) in Rust.
- **Local Database**: SQLite (via `rusqlite` with bundled compilation).
- **Cloud Backup**: Integrated manual and automatic backup/restore with Google Drive & OneDrive (OAuth TCP listener on local port `59135`).
- **Platforms**: Linux, Android, and future in Windows/iOS

---

## Coding standards & conventions
- Clean and modular code with clear separation of concerns.
- Small files and components. If a file exceeds 300 lines, consider refactoring.
- Not nested code, avoid deep nesting of conditionals and loops.
- Use descriptive variable and function names.
- Use SOLID, SRP, DRY, and KISS principles.
- Single responsibility for each function and component.

## 📂 Project Structure

- `src/`: React frontend codebase.
  - `main.tsx`: Entry point.
  - `App.tsx`: Layout orchestration and tab router.
  - `types.ts`: TypeScript data structures (`Task`, `Subtask`, `Settings`, etc.).
  - `components/`: Modular React components. Every compoent should be in its own file, and have a single responsibility, with clear props and state management, and own CSS file and classes for styling.
    - `Sidebar.tsx`: Global navigation and settings slider (persisted font size).
    - `Dashboard.tsx`: Statistics widgets.
    - `TaskCard.tsx`: Task and subtask checklist rendering.
    - `TaskModal.tsx`: CRUD task details and subtask manager.
    - `Settings.tsx`: Cloud backup accounts and forms.
    - `RestoreModal.tsx`: Sync alert dialog.
    -- Se folder of others
- `src-tauri/`: Tauri Rust backend codebase.
  - `src/main.rs`: Entry point.
  - `src/lib.rs`: Tauri setup, command handlers, and window event interception.
  - `src/db.rs`: SQLite schemas, migrations, and CRUD operations.
  - `src/backup.rs`: Google Drive and OneDrive OAuth TCP server, sync mechanisms.
  - `src/systray.rs`: System tray definitions (Open, Close, tray menu click actions).

## 🛠️ Setup & Build Commands

- **Install Dependencies**: `npm install`
- **Run in Development mode (Frontend + Backend)**: `npm run tauri dev`
- **Build Frontend production bundle**: `npm run build`
- **Build Tauri application (Production)**:
  - If building with full installers (deb, rpm): `npm run tauri build`
  - If building only the portable release binary: `export PATH="$HOME/.cargo/bin:$PATH" && cargo build --manifest-path src-tauri/Cargo.toml --release`
  *(Note: The `custom-protocol` feature must be enabled in `src-tauri/Cargo.toml` so static assets are bundled inside the executable, allowing it to run without a dev server.)*
- **Build android**: `npm run android:build`
- **Run android**: `npm run android:dev`
- **Clean build artifacts**: `cargo clean`

## 🎨 Styling & UI Conventions

1. **Vanilla CSS**: Do not install/use TailwindCSS. Add/modify global styles in `src/App.css` and `src/index.css`, but style for components go in their respective CSS files.
2. **Dark Theme**: The UI is designed as a sleek, premium dark-mode dashboard. Avoid generic light backgrounds. Use CSS variables defined in `src/index.css`.
3. **Form Controls Custom Styling**:
   - WebKitGTK on Linux forces native styling on selects and inputs unless overridden.
   - For `<select>` elements, you **must** use `appearance: none; -webkit-appearance: none;` and provide a custom SVG chevron image background so they match the dark theme and don't render as white widgets.
4. **Font-Size Scaling**:
   - The user can change the application font size via a slider in the Sidebar.
   - The value is stored in `localStorage` under `todo-font-size` and applies a CSS variable `--app-font-size` to `body`. Do not hardcode fixed pixel sizes on texts; use relative units or CSS variables where necessary.


## 📅 Date Management & Validation

1. **Database Format**: All dates in the SQLite database are stored in `YYYY-MM-DD` format (ISO 8601 date segment) to ensure correct chronological sorting and lexicographical queries (e.g. `dueDate < todayStr`).
2. **UI Format**: For Portuguese users, dates must be displayed in `DD/MM/AAAA` format.
3. **Validation**:
   - Direct manual typing of dates is enabled via a masked input in `TaskModal.tsx`.
   - Never save a date to the database without checking its calendar validity first using the `isValidDate(localDate)` helper. This function prevents impossible dates (e.g. `31/06/2026`) and handles leap years.

## 🦀 Backend (Rust) Best Practices

1. **Tokio Async Runtimes**: Tauri v2 controls its own event loop and reactor. Running async background tasks with standard `tokio::spawn` during Tauri initialization will panic due to the lack of a running Tokio reactor. **Always use `tauri::async_runtime::spawn`** for background routines.
2. **Window Interception**: Clicking the window close button (`X`) hides the window instead of killing the process, allowing the application to run in the background (systray). The process should only terminate when "Quit" is clicked in the tray.
3. **SQLite**: rusqlite is bundled. Database operations and setting configurations are handled in `db.rs`.
