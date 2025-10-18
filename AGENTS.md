# AGENTS.md

This file provides guidance for AI agents working on this project. It outlines
the project's structure, required toolchain, coding conventions, and development
workflow. Adhering to these instructions ensures consistency and high-quality
code.

## Project overview

**Project name:** XZepr

**Description:**
`A high-performance event tracking server built in Rust, featuring real-time event streaming with Redpanda, comprehensive authentication, role-based access control, and observability. Perfect for tracking CI/CD events, deployments, builds, and other system activities.`

**Architecture:** This project follows a layered architecture pattern with clean
separation of concerns:

- **Domain Layer** (`src/domain/`) - Core business logic and entities
- **Application Layer** (`src/application/`) - Use cases and application
  services
- **API Layer** (`src/api/`) - REST endpoints with middleware
- **Infrastructure Layer** (`src/infrastructure/`) - Database, Redpanda, and
  external integrations
- **Authentication Layer** (`src/auth/`) - Multi-provider auth with RBAC

## Development instructions

### Build and setup

- **Rust toolchain:** The project uses the latest stable Rust toolchain.
- **Cargo commands:**
  - To build the project: `cargo build`
  - To build in release mode: `cargo build --release`
- **Dependencies:** New dependencies should be added using
  `cargo add <crate_name>`.

### Testing

- **Running tests:** Execute the test suite with `cargo test`.
- **Test coverage:** For new features, include corresponding unit tests in the
  `src/` directory.

### Formatting and linting

- **Rustfmt:** All code must be formatted using `rustfmt`. Use `cargo fmt` to
  apply formatting.
- **Clippy:** All code must pass `clippy` checks. Use
  `cargo clippy -- -D warnings` to enforce all lints as warnings.
- **GitHub Actions (Optional):** Reference the project's CI configuration if it
  automatically runs these checks. (e.g., "The `.github/workflows/ci.yaml` file
  defines our CI process.")

### Configuration

- **YAML Files:** All yaml files end in .yaml. DO NOT USE .yml

## Coding guidelines

### Standard practices

- **Idiomatic Rust:** Adhere to the Rust API Guidelines and idiomatic patterns.
- **Error handling:** Prefer `Result<T, E>` for recoverable errors and `panic!`
  for unrecoverable bugs.
- **Ownership:** Leverage Rust's ownership system to ensure memory safety.

### Documentation

- **Documentation Framework:** Use the Diataxis Framework for Documentation
- **Doc comments:** Every public function, struct, enum, and module should have
  `///` doc comments explaining its purpose, arguments, and return values.
- **Example code:** Include code examples within your doc comments, which are
  tested by `cargo test`.
- **Internal documentation:** Use `//` comments for internal implementation
  details.
- **Markdown:** ALL MARKDOWN file names should be lower case except for the
  README.md. DO NOT USE EMOJIs. Use the rules .markdownlint.json.

### Specific patterns

- **Concurrency:**
  `<Describe how concurrency is handled, e.g., using `tokio`, `async-std`, or standard library threads.>`
- **FFI (if applicable):**
  `<If your project uses a Foreign Function Interface, detail how unsafe code blocks are managed and what safety invariants must be upheld.>`

## CI/CD process

- **Continuous Integration:**
  `<Summarize the project's CI process, for example, running tests and lints on every push to `main` or pull request.>`
- **Pull Requests:**
  `<Specify any PR guidelines, such as requiring a clean `cargo
  test` run before merging.>`

## Concrete examples

- **Reference:** For a good example of how code is written, see the file:
  `src/lib.rs`
- **Avoid:** For examples of legacy code or patterns to avoid, see the file:
  `src/old_module.rs`

## Git Conventions

### Branch Naming

- **PR branches**: Use lowercase format `pr-<jira_issue>`
  - Example: `pr-cpipe-1234`
  - Example: `pr-cpipe-5678`
- **Keep branch names lowercase** - No uppercase letters or camelCase
- **Include JIRA issue** - Always reference the ticket being worked on

### Commit Messages

All commits **MUST** follow the
[Conventional Commits](https://www.conventionalcommits.org/) specification with
the JIRA issue included at the end.

#### Format

```text
<type>(<scope>): <description> (<JIRA_ISSUE>)

[optional body]

[optional footer(s)]
```

#### Types

- **feat**: A new feature
- **fix**: A bug fix
- **docs**: Documentation only changes
- **style**: Changes that do not affect the meaning of the code (white-space,
  formatting)
- **refactor**: A code change that neither fixes a bug nor adds a feature
- **perf**: A code change that improves performance
- **test**: Adding missing tests or correcting existing tests
- **chore**: Changes to the build process or auxiliary tools

#### Commit Examples

```text
feat(auth): add user authentication module (CPIPE-1234)

fix(main): updated foo to handle edge cases (CPIPE-1234)

docs(readme): update installation instructions (CPIPE-5678)

refactor(api): simplify error handling logic (CPIPE-9012)

test(utils): add unit tests for string parser (CPIPE-3456)
```

#### Rules

- **Type is required** - Must be one of the standard types
- **Scope is optional** - Use parentheses if included
- **Description is required** - Short summary in lowercase
- **JIRA issue is required** - Must be in uppercase in parentheses at the end
- **Keep the first line under 72 characters** - Including the JIRA issue
- **Use imperative mood** - "add" not "added", "fix" not "fixed"

## Documentation Style Guide

This guide defines the style conventions for documentation in the project. All
documentation follows the **Diataxis Framework** for organization and structure.

### Documentation Organization (Diataxis Framework)

All documentation is organized into four categories:

1. **Tutorials** (`docs/tutorials/`) - Learning-oriented, hands-on lessons
2. **How-to Guides** (`docs/how_to/`) - Task-oriented, problem-solving guides
3. **Explanations** (`docs/explanations/`) - Understanding-oriented, conceptual
   discussion
4. **Reference** (`docs/reference/`) - Information-oriented, technical
   specifications

#### Structure Example

```text
docs/
├── README.md                    # Documentation overview
├── tutorials/
│   ├── README.md               # Tutorials index
│   └── getting_started.md      # Learning guide
├── how_to/
│   ├── README.md               # How-to index
│   ├── setup.md                # Installation steps
│   └── use_feature.md          # Task guide
├── explanations/
│   ├── README.md               # Explanations index
│   └── architecture.md         # Conceptual discussion
└── reference/
    ├── README.md               # Reference index
    ├── api.md                  # API specifications
    └── style_guide.md          # Standards and conventions
```

### File Naming Conventions

#### Markdown Files

- **README files**: `README.md` (uppercase)
- **All other documentation**: lowercase with underscores
  - ✅ `docs/tutorials/getting_started.md`
  - ✅ `docs/how_to/setup.md`
  - ✅ `docs/reference/api.md`
  - ❌ `docs/GETTING_STARTED.md`
  - ❌ `docs/How-To-Setup.md`

### Markdown Formatting

- **Documentation Framework:** Follow the Diataxis framework for structuring
  documentation:
  - **Tutorials** - Learning-oriented, teach through hands-on examples
  - **How-to Guides** - Task-oriented, solve specific problems
  - **Explanations** - Understanding-oriented, clarify concepts
  - **Reference** - Information-oriented, technical specifications
- **Category Placement:** Place each document in the appropriate category
  directory
- **Index Files:** Each category has a README.md index explaining its purpose
- **Doc comments:** Every public function should have doc comments explaining
  its purpose, arguments, and return values
- **Example code:** Include code examples within your doc comments
- **Internal documentation:** Use `#` comments for internal implementation
  details
- **MARKDOWN**
  - **Filenames:** ALL MARKDOWN file names should be lower case except for the
    README.md.
  - **DO NOT USE EMOJIs**
  - **Markdownlint:** Use the rules .markdownlint.json
- **Centralized documentation:** All documentation lives in `docs/` directory
  with Diataxis structure
- **Documentation updates:** Update documentation as part of the development
  process. When adding new features, create or modify documentation in the
  appropriate category
- **DO NOT USE EMOJIS in documentation or code comments**
- **Always specify language for fenced code blocks:**
  ```python
  # Python code here
  ```

### Markdownlint Configuration

The project uses `.markdownlint.json`:

```json
{
  "default": true,
  "line-length": {
    "code_blocks": false,
    "tables": false,
    "headings": false
  },
  "MD026": {
    "punctuation": ".,;:!。，；：！？"
  },
  "MD025": false,
  "MD024": false,
  "MD033": false,
  "MD036": false,
  "MD059": false
}
```

### Validation

#### Before Committing

Lint and format your markdown files:

```bash
markdownlint --fix --config .markdownlint.json docs/your_file.md
prettier --write --parser markdown --prose-wrap always docs/your_file.md
```

## Updates

This file is a living document. If you notice an AI agent making a repeated
mistake or suggesting code that doesn't align with project standards, update
this file with a new rule to prevent it.

## Prompt

You are a master Rust developer with an IQQ of 161.  Follow the rules in @AGENTS.md put the summary in docs/explanations and filename should be lowercase.
