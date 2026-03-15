# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Development
```bash
npm run dev          # Start Vite dev server (port 1420)
npm run tauri dev    # Start full Tauri desktop app with hot reload
```

### Build
```bash
npm run build        # TypeScript check + Vite build (frontend only)
npm run tauri build  # Full desktop app build (frontend + Rust backend)
```

### Frontend only
```bash
npx tsc --noEmit     # Type check without emitting files
```

## Architecture

SafeAI-Lite is a **desktop data desensitization tool** built with React/TypeScript (frontend) + Rust/Tauri (backend). It lets users redact sensitive data before sending to cloud AI services, then restore AI responses back to original form.

### Frontend (`src/`)
- **`App.tsx`** — React Router layout with Ant Design locale (zh_CN) and Zustand stores initialized
- **`pages/`** — Route-based pages: `Desensitize/` (main workflow), `EntityConfig/` (entity management), `IntentDesensitize/`, `ProxySettings/`
- **`stores/`** — Zustand stores: `entityStore.ts` (entity configs), `sessionStore.ts` (session history)
- **`services/`** — Tauri `invoke()` wrappers: `desensitizeApi.ts`, `entityApi.ts`, `sessionApi.ts`, `fileApi.ts`
- **`types/`** — Shared TypeScript interfaces for `entity.ts`, `session.ts`, `file.ts`
- Tailwind CSS with **preflight disabled** (required for Ant Design compatibility)
- Path alias `@/*` maps to `src/*`

### Backend (`src-tauri/src/`)
- **`commands/`** — Tauri command handlers (thin layer, delegates to services)
- **`services/`** — Business logic: `desensitize_service.rs`, `restore_service.rs`, `ner_service.rs`, `export_service.rs`
- **`models/`** — Rust data structs: `entity.rs`, `mapping.rs`, `session.rs`
- **`db/`** — SQLite setup; database stored at `~/Documents/SafeAI-Lite/data/safeai.db`
- **`ner/`** — Bundled ONNX model files (`model_quantized.onnx`, `tokenizer.json`, ONNX runtime DLLs)

### Core Workflow (3 stages)
1. **Scan** — Regex patterns (built-in + custom entities) + ONNX NER model (bert-base-chinese-ner, detects PER/ORG/LOC/MISC) analyze input
2. **Review** — User confirms/modifies detected items in `MappingTable` component
3. **Execute** — Generates `[EntityType_N]` placeholders, stores mappings in SQLite session; user later pastes AI response back to **Restore**

### NER Service
ONNX runtime is lazy-initialized on startup. The model runs CPU-only inference via the `ort` crate. Model files are bundled inside the Tauri app package.

### Data Persistence
SQLite tables: `sensitive_entities` (built-in + custom), `sessions`, `desensitize_mappings`. Sessions are per-desensitization and hold the mapping needed for reversal.

### File Support
- **Input**: `.doc/.docx`, `.xls/.xlsx`, `.pdf`, `.log`, `.txt`
- **Output** (restore/export): Word, Excel, PDF, TXT via `docx-rs`, `rust_xlsxwriter`, `printpdf`
