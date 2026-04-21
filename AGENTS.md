# AGENTS Guide for slimjelly

This document defines the default expectations for coding agents working in this repository.
Follow it unless a user request explicitly overrides it.

## Project Overview
- Language: Rust (edition 2024)
- Crate type: single binary crate (`src/main.rs`)
- UI stack: `eframe` + `egui`
- Async runtime: `tokio` multi-thread runtime
- HTTP: `reqwest` with `rustls-tls` and JSON support
- Serialization: `serde`, `serde_json`, `toml`
- Error model: centralized `AppError` in `src/error.rs`
- Persistence: TOML config + encrypted local session token

## Repository Layout
- `src/main.rs`: startup, Linux backend selection, runtime init, app launch
- `src/app/mod.rs`: app state, message types, `eframe::App` implementation
- `src/app/actions.rs`: async actions, API calls, playback/settings side effects
- `src/app/ui.rs`: drawing code, navigation, and message handling
- `src/app/playback.rs`: mpv IPC polling and exit watcher helpers
- `src/config.rs`: config schema/defaults, load/save logic, tests
- `src/secure_store.rs`: encrypted session token storage, tests
- `src/jellyfin/client.rs`: Jellyfin HTTP wrapper and request helpers, tests
- `src/jellyfin/models.rs`: Jellyfin DTOs (includes `MediaStreamInfo` for subtitle streams)
- `src/subtitles/client.rs`: OpenSubtitles HTTP wrapper
- `src/subtitles/models.rs`: OpenSubtitles DTOs
- `src/seerr/mod.rs`: Jellyseerr/Overseerr module re-exports
- `src/seerr/client.rs`: Jellyseerr HTTP wrapper (search, requests, status), tests
- `src/seerr/models.rs`: Jellyseerr DTOs (search results, media requests, status codes)
- `src/error.rs`: shared app error definitions and conversions
- `build.rs`: Linux runtime env embedding (`RPATH`, `XKB`)

## Build, Run, Lint, Test Commands
Run all commands from repository root.

### Build and check
- `cargo check`
- `cargo check --all-targets`
- `cargo build`
- `cargo build --release`
- NixOS-friendly build: `nix develop -c cargo build --release`

### Run
- `cargo run`
- Debug logging: `RUST_LOG=debug cargo run`
- Force X11 backend: `SLIMJELLY_UNIX_BACKEND=x11 cargo run`
- Force Wayland backend: `SLIMJELLY_UNIX_BACKEND=wayland cargo run`

### Test
- Full suite: `cargo test`
- With stdout/stderr: `cargo test -- --nocapture`
- List available tests: `cargo test -- --list`

### Run a single test (important)
- By function name: `cargo test test_function_name`
- By module path: `cargo test module::submodule::test_function_name`
- Exact match only: `cargo test test_function_name -- --exact`
- Single integration test file: `cargo test --test file_name`
- Single integration test function: `cargo test --test file_name test_function_name`

Examples from this repository:
- `cargo test app::tests::retry_transcode_when_quick_exit_and_no_progress`
- `cargo test config::tests::save_config_persists_and_roundtrips`
- `cargo test jellyfin::client::tests::normalize_base_url_rejects_empty_input`
- `cargo test secure_store::tests::stores_and_loads_session_roundtrip`
- `cargo test seerr::client::tests::normalize_url_strips_api_v1_suffix`

### Lint and format
- Format: `cargo fmt --all`
- Format check: `cargo fmt --all -- --check`
- Clippy strict: `cargo clippy --all-targets --all-features -- -D warnings`
- If needed: `rustup component add rustfmt && rustup component add clippy`

## Linux and Nix Runtime Notes
- `build.rs` reads `SLIMJELLY_RPATH_LIBS` and injects linker rpath entries.
- `build.rs` reads `SLIMJELLY_XKB_CONFIG_ROOT` and embeds a default XKB path.
- `flake.nix` sets both variables inside `devShell`.
- X11 runtime needs `libX11`.
- Wayland runtime often needs `wayland`, `libxkbcommon`, `xkeyboard-config`, `mesa`, `libglvnd`.
- If keyboard init fails under Wayland, set `XKB_CONFIG_ROOT=<path>/share/X11/xkb`.

## Code Style and Conventions

### Formatting and structure
- Always accept `rustfmt` output; do not hand-format alignment.
- Keep functions focused; split mixed-responsibility logic into helpers.
- Prefer guard clauses and early returns over deep nesting.
- Keep UI update/render methods lightweight and deterministic.
- Avoid unrelated refactors in feature or bug-fix patches.

### Imports
- Keep import groups ordered: `std`, third-party crates, then local modules.
- Avoid wildcard imports.
- Prefer explicit symbols over broad module imports.

### Types and serialization
- Use explicit types at module boundaries.
- Keep DTO fields aligned with API payload contracts.
- Use serde attributes intentionally (`rename_all`, explicit renames, skip rules).
- Use `Option<T>` only when missing values are semantically valid.
- Keep tick/time conversions explicit and unit-safe.

### Naming
- `snake_case`: functions, methods, variables, modules, files.
- `UpperCamelCase`: structs, enums, traits.
- `SCREAMING_SNAKE_CASE`: constants.
- Prefer descriptive names (`play_session_id`, `selected_playlist_id`, etc.).

### Error handling
- Use `Result<T, AppError>` for fallible operations.
- Convert external errors via `From` or explicit mapping with context.
- Include actionable context in error strings.
- Map non-success HTTP responses to `AppError::ApiStatus`.
- Avoid `unwrap()`/`expect()` in production paths.

### Async and concurrency
- Never block the UI thread for network, file I/O, or process operations.
- Spawn async tasks on Tokio and return results through `UiMessage`.
- Do not hold mutex guards across `.await`.
- Use cancellation/generation guards for long-lived background loops.

### API and security rules
- Add Jellyfin endpoints in `src/jellyfin/client.rs`; DTOs in `src/jellyfin/models.rs`.
- Add OpenSubtitles endpoints in `src/subtitles/client.rs`; DTOs in `src/subtitles/models.rs`.
- Add Jellyseerr endpoints in `src/seerr/client.rs`; DTOs in `src/seerr/models.rs`.
- Keep auth header and token handling centralized in client helpers.
- Treat access tokens, passwords, and API keys as secrets.
- Never log tokens, auth headers, or signed URLs.
- Keep self-signed TLS support explicit and opt-in.

### Testing guidance
- Add unit tests near changed logic (`#[cfg(test)]` modules).
- Keep tests deterministic; avoid live network in unit tests.
- Prefer behavior-based test names.
- For secure store changes, include roundtrip and corrupted input cases.
- For URL/request changes, test normalization and query/header behavior.

## Agent Change Scope
- Make the smallest safe change that solves the requested task.
- Preserve established architecture and patterns unless asked otherwise.
- Update docs if behavior or commands change.
- Never revert unrelated local changes you did not create.

## Cursor and Copilot Instruction Files
Checked paths:
- `.cursorrules`
- `.cursor/rules/`
- `.github/copilot-instructions.md`

Current status: no Cursor or Copilot instruction files were found.
If these files are added later, treat them as authoritative and merge their guidance with this file.
If instructions conflict, follow the most specific repository-local instruction file.
