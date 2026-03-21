# AGENTS Guide for slimjelly

This file is the working contract for coding agents in this repository.
Use it as the default implementation standard for all edits.

## Repository Snapshot

- Language: Rust (edition 2024)
- App type: native desktop GUI (`eframe`/`egui`) + async runtime (`tokio`)
- Crate type: single binary crate (`src/main.rs`)
- Networking: `reqwest` with `rustls`
- Serialization: `serde`, `serde_json`, `toml`
- Error model: `AppError` in `src/error.rs` via `thiserror`

## Source Layout

- `src/main.rs`: app startup, runtime init, window boot
- `src/app.rs`: UI state machine and screen logic
- `src/config.rs`: config types/defaults + TOML persistence
- `src/secure_store.rs`: encrypted local session handling
- `src/jellyfin/client.rs`: Jellyfin HTTP client and endpoint wrappers
- `src/jellyfin/models.rs`: request/response DTOs
- `src/error.rs`: central error enum and conversions

## Build, Run, Lint, Test

Run all commands from repository root.

### Build / Check

- `cargo check`
- `cargo check --all-targets`
- `cargo build --release`

Nix/Wayland recommended build:

- `nix develop -c cargo build --release`
- This bakes runtime RPATH/XKB defaults via build env vars

### Run

- `cargo run`
- `RUST_LOG=debug cargo run`

Linux display backend notes:

- Auto-detect (default): `cargo run`
- Force X11: `SLIMJELLY_UNIX_BACKEND=x11 cargo run`
- Force Wayland: `SLIMJELLY_UNIX_BACKEND=wayland cargo run`
- `WINIT_UNIX_BACKEND` is also respected if `SLIMJELLY_UNIX_BACKEND` is unset

Linux runtime library notes:

- X11 backend requires `libX11` at runtime
- Wayland backend requires `libwayland-client` at runtime
- On NixOS, ensure these libraries are present in your shell/system closure (e.g. `xorg.libX11`, `wayland`)

Build-time runtime embedding notes:

- `build.rs` reads `SLIMJELLY_RPATH_LIBS` to inject linker rpath entries
- `build.rs` reads `SLIMJELLY_XKB_CONFIG_ROOT` to embed default `XKB_CONFIG_ROOT`
- `flake.nix` sets these automatically in `devShell`

Wayland extra runtime notes:

- Wayland keyboard initialization may require `libxkbcommon` + `xkeyboard-config`
- If needed, set `XKB_CONFIG_ROOT=<store-path>/share/X11/xkb`
- OpenGL path may require `mesa` + `libglvnd` for GL/EGL config discovery

### Tests

- Full suite: `cargo test`
- With output: `cargo test -- --nocapture`

### Run One Test (important)

- By function name: `cargo test test_function_name`
- By module path: `cargo test module::submodule::test_function_name`
- One integration file: `cargo test --test integration_test_file`
- One integration test: `cargo test --test integration_test_file test_function_name`
- Exact name only: `cargo test test_function_name -- --exact`

### Lint / Format

- Format: `cargo fmt --all`
- Format check: `cargo fmt --all -- --check`
- Clippy strict: `cargo clippy --all-targets --all-features -- -D warnings`

If `fmt`/`clippy` are missing:

- `rustup component add rustfmt`
- `rustup component add clippy`

## Coding Style Rules

### Formatting and Structure

- Always use rustfmt output; do not hand-format alignment.
- Keep functions focused; split large mixed-responsibility functions.
- Prefer guard clauses and early returns over deep nesting.
- Keep multiline literals/calls trailing-comma friendly.
- Avoid giant `match` branches that do unrelated work; extract helpers.

### Imports

- Import groups should be ordered:
  1. `std`
  2. third-party crates
  3. local crate modules (`crate::...`, `super::...`)
- Avoid wildcard imports.
- Prefer explicit imports for commonly used symbols.

### Naming Conventions

- `snake_case`: functions, methods, vars, modules, files.
- `UpperCamelCase`: structs/enums/traits.
- `SCREAMING_SNAKE_CASE`: constants/statics.
- Use explicit names (`play_session_id`, `selected_view_id`, etc.).

### Types and Serialization

- Use explicit types at module boundaries.
- Keep DTOs aligned with Jellyfin payload contracts.
- Use serde attributes intentionally (`rename_all`, field renames).
- Use `Option<T>` only when absence is valid in server/domain semantics.

### Error Handling

- Use `Result<T, AppError>` for fallible app operations.
- Map external errors via `From` or explicit conversion with context.
- Include actionable context in messages (`what failed` + reason).
- Avoid `unwrap()`/`expect()` in production code paths.
- Reserve panic for impossible states only.

### Async and Concurrency

- Never block the UI thread with network/IO/process work.
- Spawn async work on Tokio and communicate through app messages.
- Do not hold mutex guards across `.await`.
- Make background loops cancellable/generation-gated where possible.

### Jellyfin Client Rules

- Add endpoint wrappers in `src/jellyfin/client.rs`.
- Add DTO changes in `src/jellyfin/models.rs`.
- Normalize and validate server URL input before requests.
- Check HTTP status and map non-success to `AppError::ApiStatus`.
- Keep auth header behavior and token handling centralized.

### Security and Secrets

- Treat access tokens/passwords as secrets.
- Persist sessions via `secure_store` only.
- Do not log tokens, auth headers, or signed stream URLs.
- Keep self-signed TLS behavior opt-in and explicit.

### Logging

- Use `log` macros (`debug!`, `info!`, `warn!`, `error!`).
- Keep logs concise and diagnostic, not noisy.
- Never emit sensitive user/server credentials.

### UI Behavior

- Keep `egui` `update()` cheap and deterministic.
- Put expensive work in helpers/tasks; report status to UI.
- Show progress/status for long operations.
- Hide admin-only controls for non-admin sessions.

## Testing Guidance

- Add unit tests near logic-heavy modules with `#[cfg(test)]`.
- Keep tests deterministic; avoid live network in unit tests.
- Isolate URL construction and payload serialization for testability.
- For secure store changes, test roundtrip + corrupt input handling.
- Prefer behavior-oriented names (e.g. `loads_session_when_file_valid`).

## Change Scope Rules for Agents

- Make the smallest safe change that solves the task.
- Do not refactor unrelated code in the same patch.
- Preserve existing patterns unless asked to redesign.
- Update docs/config docs when behavior changes.

## Cursor / Copilot Rules

Checked paths:

- `.cursorrules`
- `.cursor/rules/`
- `.github/copilot-instructions.md`

Current status: no Cursor or Copilot instruction files were found.
If those files are added later, treat them as authoritative and merge with this guide.
