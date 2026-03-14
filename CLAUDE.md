# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A Zed editor extension that adds a `Kubernetes` language mode on top of Tree-sitter YAML, backed by `yaml-language-server`. It auto-detects Kubernetes manifests by file suffix (`*.k8s.yaml`, `*.kubernetes.yml`) and by best-effort first-line matching when a buffer starts with `apiVersion:`. Generic `.yaml` files still belong to built-in `YAML` unless the user remaps `file_types` or manually switches the language. Ships an optional icon theme overlay.

## Build and check commands

The repo uses `mise` for task automation. Run `mise trust .mise.toml` once after cloning.

```sh
mise run check          # full local gate (test + lint + packaging)
mise run test           # cargo test + cargo nextest
mise run lint           # fmt --check, clippy, query formatting, swift formatting
mise run build:wasm     # rebuild extension.wasm after Rust changes
mise run package        # verify packaging with zed-extension CLI
mise run check:queries  # validate Tree-sitter query files
```

Individual Rust commands:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --workspace --all-targets --no-fail-fast
```

`.mise.toml` redirects `CARGO_HOME` and `RUSTUP_HOME` into `tmp/`, so Rust toolchains and crates live inside the project tree.

## Zed validation workflow

```sh
mise run zed:install-dev-extension   # install into isolated Zed profile
mise run zed:sync-extension          # rotate stale runtime cache
mise run zed:deployment              # open deployment fixture in isolated profile
mise run zed:foreground <path>       # launch Zed with app logs visible
```

The manual validation checklist in README.md covers fixture-based checks across `fixtures/valid/`, `fixtures/invalid/`, `fixtures/embedded/`, and `fixtures/templates/`. Run it after the automated checks pass.

## Architecture

The extension builds as a single `cdylib` WebAssembly crate with a thin entrypoint plus focused helper modules:

- `src/kubernetes.rs` - thin extension entrypoint, implements `zed::Extension` trait, routes to language server and settings modules. Integration tests for `extension.toml`, `first_line_pattern` detection, and icon theme live here.
- `src/language_server.rs` - resolves `yaml-language-server` binary (user-configured path > `$PATH` lookup > managed npm install). Builds the `zed::Command` with merged args and env.
- `src/settings.rs` - owns the curated-plus-raw settings model for `kubernetes-language-server`: extension defaults, `settings.kubernetes`, raw `settings.yaml`, path resolution for schema associations, YAML injection config, and the LSP settings schemas exposed to Zed's Settings Editor.
- `src/helm_language_server.rs` - resolves the Helm language server binary (user-configured path > `$PATH` lookup > downloaded GitHub release).
- `src/context_server.rs` - resolves the Kubernetes context server command and exposes typed `settings_schema` plus `default_settings` for the context-server UI.
- `src/docs.rs` and `src/templates.rs` - provide docs indexing/explanation helpers and slash-command manifest templates.

Language definition lives in `languages/kubernetes/config.toml` with Tree-sitter query files (`highlights.scm`, `outline.scm`, etc.) alongside it. The `first_line_pattern` regex does the plain-YAML Kubernetes auto-detection within Zed's 256-character content window.

`extension.toml` declares the extension metadata, grammar source, npm capability for `yaml-language-server`, and the language server ID `kubernetes-language-server`.

## Configuration surfaces

- For Kubernetes-mode buffers, user and project overrides come from `lsp.kubernetes-language-server.binary`, `lsp.kubernetes-language-server.settings`, and `lsp.kubernetes-language-server.initialization_options`.
- `src/language_server.rs` reads the `binary` block first, then falls back to `$PATH`, then to the managed npm install. `src/kubernetes.rs` passes `settings` and `initialization_options` through the Zed extension hooks.
- `src/settings.rs` treats `lsp.kubernetes-language-server.settings.kubernetes` as the extension-owned layer and `lsp.kubernetes-language-server.settings.yaml` as raw `yaml-language-server` passthrough for Kubernetes-mode buffers. Precedence is `extension defaults < settings.kubernetes < settings.yaml`.
- The curated `settings.kubernetes` block currently exposes `includeDefaultSchemas`, `injectIntoYamlLanguageServer`, and `schemaAssociations`.
- Relative schema paths in both `settings.kubernetes.schemaAssociations` and raw `settings.yaml.schemas` resolve against the worktree root. `~/...` resolves against `HOME`.
- Generic `.yaml` buffers that still belong to built-in `YAML` are configured through `lsp.yaml-language-server`, not `kubernetes-language-server`. This extension only mirrors extension-owned schema associations into `yaml-language-server`, controlled by `settings.kubernetes.injectIntoYamlLanguageServer`. Raw `settings.yaml` never leaks into built-in YAML.
- `src/kubernetes.rs` now implements `language_server_workspace_configuration_schema` and `language_server_initialization_options_schema`. The Kubernetes schema is typed for the extension-owned block plus permissive raw passthrough objects; Helm exposes a minimal typed wrapper with a raw `helm-ls` block.
- `helm-language-server` is opt-in at the language level via `languages.Kubernetes.language_servers`. Its runtime config stays raw pass-through under `lsp.helm-language-server.settings["helm-ls"]`.
- `src/context_server.rs` is the reference pattern for a fully typed configuration surface in this repo because it returns `settings_schema` and `default_settings` for `kubernetes-context-server`.

## Key terms

- **content window** - the first 256 characters of a buffer that Zed exposes to `first_line_pattern` for language detection (`languages/kubernetes/config.toml:5`)
- **worktree** - Zed's representation of an open project directory, passed to extension trait methods for per-project settings
- **schema globs** - the file patterns in `settings.rs` that tell `yaml-language-server` which files get Kubernetes schema validation
- **managed npm install** - the fallback path in `language_server.rs` where the extension installs `yaml-language-server` via `zed::npm_install_package` when no local binary is found

## Gotchas

- After any Rust change, you must run `mise run build:wasm` to rebuild `extension.wasm` before testing in Zed - the checked-in wasm is the artifact Zed loads, not a cargo build output
- `.mise.toml` sets `CARGO_HOME=tmp/cargo` and `RUSTUP_HOME=tmp/rustup`, so toolchains and crates live inside the project tree. Direct `cargo` invocations outside `mise` use your global toolchain instead
- `Cargo.toml` intentionally pins `zed_extension_api` to the Zed git rev `07cfa81f09520c691715c40acff84994a55acaf3` because crates.io `0.7.0` still lacks the schema-hook methods used by this repo. If future work touches extension settings hooks, check whether Zed has published a newer crate before changing that pin.
- The `first_line_pattern` regex in `config.toml` uses `(?m)` multiline mode and `\A` anchor - test changes with `cargo nextest run` before manual Zed validation, since the Rust test suite exercises real fixture files against the compiled regex
- `zed:sync-extension` moves the isolated profile's `extensions/work/kubernetes` aside to force a fresh language-server install. Without this, stale cached state survives across `build:wasm` rebuilds

## Pre-commit workflow

Run before every commit: `mise run check && mise run package`

## Conventions

- Conventional Commits with scope: `feat(extension): ...`, `fix(settings): ...`
- Sign commits: `git commit -sS`
- Tests go in the same file as the code they cover, with descriptive names like `managed_node_command_places_script_path_before_server_arguments`
- No `unwrap()` in production code
- Swift helpers under `.mise/` use `.swift-format` with 2-space indentation
