# AGENTS.md

This file provides guidance for AI agents working on this project. It outlines the project's structure, required toolchain, coding conventions, and development workflow. Adhering to these instructions ensures consistency and high-quality code.

## Project overview

**Project name:** XZepr

**Description:** `A high-performance event tracking server built in Rust, featuring real-time event streaming with Redpanda, comprehensive authentication, role-based access control, and observability. Perfect for tracking CI/CD events, deployments, builds, and other system activities.`

**Architecture:** This project follows a layered architecture pattern with clean separation of concerns:

- **Domain Layer** (`src/domain/`) - Core business logic and entities
- **Application Layer** (`src/application/`) - Use cases and application services
- **API Layer** (`src/api/`) - REST endpoints with middleware
- **Infrastructure Layer** (`src/infrastructure/`) - Database, Redpanda, and external integrations
- **Authentication Layer** (`src/auth/`) - Multi-provider auth with RBAC

## Development instructions

### Build and setup

- **Rust toolchain:** The project uses the latest stable Rust toolchain.
- **Cargo commands:**
  - To build the project: `cargo build`
  - To build in release mode: `cargo build --release`
- **Dependencies:** New dependencies should be added using `cargo add <crate_name>`.

### Testing

- **Running tests:** Execute the test suite with `cargo test`.
- **Test coverage:** For new features, include corresponding unit tests in the `src/` directory.

### Formatting and linting

- **Rustfmt:** All code must be formatted using `rustfmt`. Use `cargo fmt` to apply formatting.
- **Clippy:** All code must pass `clippy` checks. Use `cargo clippy -- -D warnings` to enforce all lints as warnings.
- **GitHub Actions (Optional):** Reference the project's CI configuration if it automatically runs these checks. (e.g., "The `.github/workflows/ci.yaml` file defines our CI process.")

### Configuration

- **YAML Files:** All yaml files end in .yaml. DO NOT USE .yml

## Coding guidelines

### Standard practices

- **Idiomatic Rust:** Adhere to the Rust API Guidelines and idiomatic patterns.
- **Error handling:** Prefer `Result<T, E>` for recoverable errors and `panic!` for unrecoverable bugs.
- **Ownership:** Leverage Rust's ownership system to ensure memory safety.

### Documentation

- **Documentation Framework:** Use the Diataxis Framework for Documentation
- **Doc comments:** Every public function, struct, enum, and module should have `///` doc comments explaining its purpose, arguments, and return values.
- **Example code:** Include code examples within your doc comments, which are tested by `cargo test`.
- **Internal documentation:** Use `//` comments for internal implementation details.
- **Markdown:** ALL MARKDOWN file names should be lower case except for the README.md. DO NOT USE EMOJIs. Use the rules .markdownlint.json.

### Specific patterns

- **Concurrency:** `<Describe how concurrency is handled, e.g., using `tokio`, `async-std`, or standard library threads.>`
- **FFI (if applicable):** `<If your project uses a Foreign Function Interface, detail how unsafe code blocks are managed and what safety invariants must be upheld.>`

## CI/CD process

- **Continuous Integration:** `<Summarize the project's CI process, for example, running tests and lints on every push to `main` or pull request.>`
- **Pull Requests:** `<Specify any PR guidelines, such as requiring a clean `cargo test` run before merging.>`

## Concrete examples

- **Reference:** For a good example of how code is written, see the file: `src/lib.rs`
- **Avoid:** For examples of legacy code or patterns to avoid, see the file: `src/old_module.rs`

## Updates

This file is a living document. If you notice an AI agent making a repeated mistake or suggesting code that doesn't align with project standards, update this file with a new rule to prevent it.
