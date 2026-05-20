# Project Context: Nullae

## Overview
Nullae is a Rust-based project that strictly follows specific architectural and coding conventions. It uses a CQRS pattern and emphasizes type safety, error handling, and a clear separation of concerns.

For a detailed project vision, technical stack, and functional requirements, please refer to [doc/vision.md](doc/vision.md).

## Core Conventions Reference
All development MUST strictly adhere to the comprehensive coding standards and workflow rules defined in [doc/conventions.md](doc/conventions.md).

Key guidelines that MUST be followed at all times:
- **Module Structure**: Do NOT use `mod.rs`. Use named module files to declare submodules (e.g. `src/utils.rs` declares submodules inside `src/utils/`).
- **Error Handling**: Use `anyhow!` and `bail!` macros for all errors. Do NOT add new variants to the `error::Error` enum.
- **Git Workflow**:
  - Ask for explicit permission BEFORE running `git commit`.
  - Do NOT run `git push` without explicit user request.
  - Commit messages MUST be in English only, formatted as `<type>: <description>`.
- **Documentation**: Do NOT create any new documentation files (README, Guides, etc.) unless the user explicitly requests it. Always ask first.
- **Execution**: Do NOT run backend or frontend servers on your own. Provide the user with instructions and ask them to run them.

## Development Focus
- Follow existing patterns for new entities.
- Ensure `Send + Sync` for async types.
- Validate data at module boundaries.
