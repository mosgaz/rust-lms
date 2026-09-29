## Спецификация Технического Задания: Руководство по контрибьютингу
Файл спецификации: CONTRIBUTING.md
## About this repository
This is a monorepo for the modular LMS workspace, built on top of a highly performant full-stack Rust architecture using Axum/Actix-web for the backend, Leptos 0.7+ and Tailwind CSS for the frontend, and PostgreSQL with Row-Level Security (RLS) for strict data isolation.

## Architectural Rules to Follow

* Strict Multi-tenancy: Every query modifying or reading organization data must enforce Row-Level Security via tenant_id session context. Never bypass RLS in tenant-specific business logic.
* WASM-Isomorphic Contours: lms-core-frontend compiles strictly to wasm32-unknown-unknown. It must never import crates with native OS-level dependencies (lms-core-backend, lms-database-db, lms-task-worker).
* Immutable Tracking (LRS): Offline progress synchronization must follow the immutable fact principle. Never rewrite or use destructive resolution (like Last Write Wins) on xAPI Statements.

------------------------------
## Development## Clone and setup

git clone git@github.com:mosgaz/rust-lms.git
cd rust-lms
# Install essential full-stack Rust build tools
cargo install --locked cargo-leptos
cargo install --locked leptosfmt
rustup target add wasm32-unknown-unknown

## Requirements

* Rust Stable (Refer to rust-toolchain.toml for the current exact version).
* Tailwind CSS available in your system PATH.
* PostgreSQL 15+ with TimescaleDB extension or a running local Docker environment.

## Running the Infrastructure locally
To spin up the foundational data layer and object storage (MinIO) for development:

docker compose -f specs/DEPLOY.md up -d nexus-postgres-core nexus-dam-storage

## Running the Live Development Environment
To start the full-stack hot-reload server (Backend API + Frontend Hydration) managed by cargo-leptos:

cd crates/lms-core-backend
cargo leptos watch

------------------------------
## Verification & Formatting
We maintain extremely high code quality standards enforced via compile-time constraints and strict CI linting. Always run the validation pipeline before submitting your pull request.
## 1. Code Style and Formatters
Always run both formatters before committing changes to ensure standard Rust layout and clean Leptos component trees:

cargo fmt --all && leptosfmt **/*.rs

## 2. Static Analysis and Linters
Code must compile with zero warnings or lints across all targets:

cargo clippy --workspace --all-targets --all-features -- -D warnings

## 3. Test Workspace
Run all unit, integration, and geometry/ETL snapshot tests:

cargo test --workspace

Note: Any changes made to lms-etl-mapper or lms-lrs-analytics require high data-density snapshot validation coverage.
------------------------------
## Commit Convention (Conventional Commits)
We strictly adhere to the Conventional Commits specification for automatic generation of semantic versioning and formation of CHANGELOG.md. Every commit must follow the template: category(scope): message format for automated changelog generation:

Allowed Commit Categories:

| Category | When to use |
|---|---|
| feat | New platform capability, core block, or core feature |
| fix | Bug fix (e.g., frontend rendering bug, analytical query fix) |
| docs | Documentation modifications in specs/* or code comments |
| refactor | Code restructuring without altering backend API or UX |
| build | Workspace dependencies or Cargo.toml updates |
| test | Adding, updating, or expanding snapshot/unit tests |
| ci | CI pipeline configuration or Dockerfile modifications |
| chore | Housekeeping, formatting, lint resolution |

Scopes of Responsibility:

* shared — changes to data structures or xAPI contracts.
* ui — modification of the shared design system and atomic components.
* api — changes to the server DBMS/LRS layer.
* website / student / cpanel — changes to logic inside the isomorphic application contours of client.
* server — changes to the Axum entry point or Tokio initialization.

Examples of Valid Commit Messages:

* feat(cpanel): add dynamic provisioning endpoint for new tenants
* fix(student): resolve indexeddb message duplicate on offline sync reconnect
* docs(specs): integrate commit and pull request conventions document

------------------------------
## Pull Request Guidelines

   1. Get started: Check specs/AGENTS.md & specs/CODING_STANDARDS.md: Ensure your implementation respects the hard rules (no unwrap(), strict tracing logs instead of println!, explicit RLS contexts).
   2. Branch creation: Fork the repository and create your feature branch: git checkout -b feat/tenant-sso-config
   3. Test coverage: Implement tests alongside your functionality (keep crate coverage ≥ 80%).
   4. Local verification: Format, lint, and run tests locally via the commands listed above.
   5. PR formatting: The Pull Request description must clearly state the essence of the changes, the impact on multi-tenant isolation, and the behavior of Offline-First IndexedDB queues.
   6. Finish: Submit your PR with a thorough explanation of changes, describing how multitenancy and offline-first boundaries are affected.


