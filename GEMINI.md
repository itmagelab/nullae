# Project Context: Nullae

## Overview
Nullae is a Rust-based project that strictly follows specific architectural and coding conventions. It uses a CQRS pattern and emphasizes type safety, error handling, and a clear separation of concerns.

## Technology Stack
- **Backend**: Rust (Async I/O, Tokio/Actix/Axum implied by "async" and "tracing")
- **Frontend**: Yew (Rust framework), Tailwind CSS (via CDN), Font Awesome
- **Data/Content**: YAML files in `static/` (parsed via Serde)
- **Logging**: `tracing` crate

## Critical Rules & Conventions

### 1. Code Philosophy (Strict)
- **YAGNI**: Implement ONLY what is requested. No "nice-to-have" features, no extra UI elements.
- **Minimalism**: Do not add examples or demo functionality unless asked.
- **Safety**:
  - **NO** `unwrap()` or `expect()`. Use `Result<T>` and the `?` operator.
  - **MUST** use `anyhow!` and `bail!` for errors.
  - **MUST** provide context for errors via `.context()`.

### 2. Architecture
- **CQRS**: Strictly follow Command/Query Responsibility Segregation.
- **Context**: Pass `ctx` through all application layers.
- **Configuration**: No hardcoded config; use environment variables.
- **Modules**:
  - No `mod.rs` files.
  - Use the pattern: `folder/` + `folder.rs` (declaring submodules).
  - Explicit imports (e.g., `use crate::utils::api;`).

### 3. Frontend (Yew)
- **Components**: Only use Yew. No `index.html` for styles/components.
- **Styling**: Tailwind CSS is mandatory.
- **Content**: Stored in `static/content.yaml`, loaded via `gloo-net` asynchronously.
- **State**: Handle loading states; use `use_effect_with` + `spawn_local`.

### 4. Naming & Style
- **Files/Modules**: `snake_case`. One main type per file.
- **Functions**: `verb_noun` (e.g., `create_transaction`).
- **Variables**: Nouns (e.g., `user_id`).
- **Types**: Use domain types (e.g., `TxType`), not raw strings/ints.

### 5. Error Handling
- **Errors**: Use `anyhow` for app-level errors.
- **Enum**: Do not add new variants to `error::Error` enum unless strictly necessary for control flow.
- **Logging**: Log via `tracing`: `error!` (critical), `warn!` (unexpected), `info!` (ops), `debug!` (diagnostics).

### 6. Testing
- **Unit**: Test business logic, mock external deps.
- **Integration**: Test critical user flows with a test DB; clean up after tests.

### 7. Git & Workflow
- **Commits**: English only. Format: `<type>: <description>` (e.g., `feat: add user auth`).
- **Permissions**:
  - **NEVER** commit, push, or run servers without explicit user permission.
  - Ask before creating documentation (README, etc.).

## Development Focus
- Follow existing patterns for new entities.
- Ensure `Send + Sync` for async types.
- Validate data at module boundaries.
