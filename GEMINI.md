# Project Context: Nullae

## Overview
Nullae is a Rust-based project that strictly follows specific architectural and coding conventions. It uses a CQRS pattern and emphasizes type safety, error handling, and a clear separation of concerns.

For a detailed project vision, technical stack, and functional requirements, please refer to [doc/vision.md](doc/vision.md).

For comprehensive coding standards, architectural constraints, naming conventions, error handling, testing, configuration, performance, documentation, and Git practices, please refer to [doc/conventions.md](doc/conventions.md).

## Development Focus
- Follow existing patterns for new entities.
- Ensure `Send + Sync` for async types.
- Validate data at module boundaries.
