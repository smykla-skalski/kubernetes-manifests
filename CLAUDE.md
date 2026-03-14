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
mise run zed:prepare-next-dev-source # generate the dev source tree under /tmp whose plain cargo build defaults to next
mise run zed:install-next-extension  # install the generated next-feature dev extension into a Zed profile
mise run zed:install-nightly         # install Zed Nightly itself through the official installer
mise run zed:next <path>             # install Nightly if needed, replace the real Nightly-profile dev extension, and launch it
mise run zed:sync-extension          # rotate stale runtime cache
mise run zed:deployment              # open deployment fixture in isolated profile
mise run zed:foreground <path>       # launch Zed with app logs visible
```

The manual validation checklist in README.md covers fixture-based checks across `fixtures/valid/`, `fixtures/invalid/`, `fixtures/embedded/`, and `fixtures/templates/`. Run it after the automated checks pass.

## Architecture

The extension builds as a single `cdylib` WebAssembly crate with a thin entrypoint plus focused helper modules:

- `src/kubernetes.rs` - thin extension entrypoint, implements `zed::Extension` trait, routes to language server and settings modules. Integration tests for `extension.toml`, `first_line_pattern` detection, and icon theme live here.
- `src/language_server.rs` - resolves `yaml-language-server` binary (user-configured path > `$PATH` lookup > managed npm install). Builds the `zed::Command` with merged args and env.
- `src/settings.rs` - owns the curated-plus-raw settings model for `kubernetes-language-server`: extension defaults, `settings.kubernetes`, raw `settings.yaml`, path resolution for schema associations, YAML injection config, and the `next`-only LSP settings schemas exposed to Zed's Settings Editor.
- `src/helm_language_server.rs` - resolves the Helm language server binary (user-configured path > `$PATH` lookup > downloaded GitHub release).
- `src/context_server.rs` - resolves the Kubernetes context server command and exposes typed `settings_schema` plus `default_settings` for the context-server UI.
- `src/docs.rs` and `src/templates.rs` - provide docs indexing/explanation helpers and slash-command manifest templates.

Language definition lives in `languages/kubernetes/config.toml` with Tree-sitter query files (`highlights.scm`, `outline.scm`, etc.) alongside it. The `first_line_pattern` regex does the plain-YAML Kubernetes auto-detection within Zed's 256-character content window.

`extension.toml` declares the extension metadata, grammar source, npm capability for `yaml-language-server`, and the language server ID `kubernetes-language-server`.

## Configuration surfaces

- For Kubernetes-mode buffers, user and project overrides come from `lsp.kubernetes-language-server.binary`, `lsp.kubernetes-language-server.settings`, and `lsp.kubernetes-language-server.initialization_options`.
- `src/language_server.rs` reads the `binary` block first, then falls back to `$PATH`, then to the managed npm install. `src/kubernetes.rs` passes `settings` and `initialization_options` through the Zed extension hooks.
- `src/settings.rs` treats `lsp.kubernetes-language-server.settings.kubernetes` as the extension-owned layer and `lsp.kubernetes-language-server.settings.yaml` as raw `yaml-language-server` passthrough for Kubernetes-mode buffers. Precedence is `extension defaults < settings.kubernetes < settings.yaml`.
- The curated `settings.kubernetes` block now covers the main YAML/Kubernetes UX knobs: schema defaults/injection, schema store, CRD store, validation, completion, formatting, style, editor defaults, custom tags, key ordering, and extra schema associations.
- Relative schema paths in both `settings.kubernetes.schemaAssociations` and raw `settings.yaml.schemas` resolve against the worktree root. `~/...` resolves against `HOME`, and `src/settings.rs` converts those local schema paths to `file://` URLs before they go to YAML LS.
- Generic `.yaml` buffers that still belong to built-in `YAML` are configured through `lsp.yaml-language-server`, not `kubernetes-language-server`. This extension only mirrors extension-owned schema associations into `yaml-language-server`, controlled by `settings.kubernetes.injectIntoYamlLanguageServer`. Raw `settings.yaml` never leaks into built-in YAML.
- Raw `settings.yaml` is still the escape hatch for niche upstream options that the curated layer does not model yet, such as proxy-related YAML LS settings.
- `src/kubernetes.rs` implements `language_server_workspace_configuration_schema` and `language_server_initialization_options_schema` only under the Cargo feature `next`. The Kubernetes schema is typed for the extension-owned block plus permissive raw passthrough objects; Helm exposes a typed `helm-ls` wrapper there too.
- `helm-language-server` is opt-in at the language level via `languages.Kubernetes.language_servers`. Its runtime config stays raw pass-through under `lsp.helm-language-server.settings["helm-ls"]`.
- `src/context_server.rs` is the reference pattern for a fully typed configuration surface in this repo because it returns `settings_schema` and `default_settings` for `kubernetes-context-server`.
- Zed's extension-card `Configure` button only exists because this extension provides `context_servers`, and it opens the MCP/context-server modal for `context_servers.kubernetes-context-server.*`. It does not surface `lsp.kubernetes-language-server.*`; those editor and LSP settings still live in Zed's Settings Editor or `settings.json`.

## Key terms

- **content window** - the first 256 characters of a buffer that Zed exposes to `first_line_pattern` for language detection (`languages/kubernetes/config.toml:5`)
- **worktree** - Zed's representation of an open project directory, passed to extension trait methods for per-project settings
- **schema globs** - the file patterns in `settings.rs` that tell `yaml-language-server` which files get Kubernetes schema validation
- **managed npm install** - the fallback path in `language_server.rs` where the extension installs `yaml-language-server` via `zed::npm_install_package` when no local binary is found

## Gotchas

- After any Rust change, you must run `mise run build:wasm` to rebuild `extension.wasm` before testing in Zed - the checked-in wasm is the artifact Zed loads, not a cargo build output
- `.mise.toml` sets `CARGO_HOME=tmp/cargo` and `RUSTUP_HOME=tmp/rustup`, so toolchains and crates live inside the project tree. Direct `cargo` invocations outside `mise` use your global toolchain instead
- `Cargo.toml` now supports two API tracks: the default build uses the released crates.io `zed_extension_api`, while `--features next` enables the git-pinned unreleased API.
- The stock "Install Dev Extension" flow always recompiles the Rust extension with plain `cargo build` and no feature flags. To test `next`, use `mise run zed:install-next-extension`, which generates `/tmp/zed-k8s-next-dev-source` with `default = ["next"]` in its Cargo manifest, compiles the full extension payload so `extension.wasm` and `grammars/yaml.wasm` already exist, and then installs that generated directory as the dev-extension symlink.
- `mise run zed:install-built-extension --features next` is still useful for packaging-style validation, but Zed's Extensions UI does not treat that copied directory install as a dev extension.
- The managed YAML LS fallback in `src/language_server.rs` currently patches the installed `yaml-language-server` runtime files to remove the invalid `[yaml]` `scopeUri: "null"` request. Upstream `main` already dropped that request, but the latest published npm package still had it during this investigation, and Zed logs it as `relative URL without a base`.
- `zed --nightly` on macOS only works when LaunchServices can resolve an actual `Zed Nightly.app`. The CLI code asks `osascript` for `path to application "Zed Nightly"` and then runs that bundle's `Contents/MacOS/cli`.
- The same macOS background launch path ignores custom `--user-data-dir` values. Do not promise isolated `/tmp` profiles through `zed --nightly`; if you must keep `--nightly`, target the real Nightly profile instead.
- When `zed:next` targets the real Nightly profile on macOS, config still comes from `~/.config/zed/settings.json`, not `~/Library/Application Support/Zed/config/settings.json`. Restricted Mode still applies, so Nightly will log `Waiting for worktree ... to be trusted` and will not start `kubernetes-language-server` or `helm-language-server` until the repo is trusted or `nightly.session.trust_all_worktrees` is enabled in the real settings file. The task should not set `ZED_STATELESS`; that env var forces Zed onto an in-memory DB and wipes persisted trusted-worktree state between launches.
- `mise run zed:install-nightly` uses Zed's official install script with `ZED_CHANNEL=nightly` to install that real Nightly app bundle when it is missing.
- `mise run zed:next [path]` is the single-command happy path for the unreleased API: it ensures Zed Nightly exists, restarts any already running Nightly instance, replaces the extension in the real Nightly profile with the generated next dev extension, clears repo-relative `CARGO_HOME` and `RUSTUP_HOME` before launch, and launches `zed --nightly` against that same profile.
- On this machine, keep `~/.local/bin/zed` pointed at Zed Nightly. Do not recreate the old `~/usr/local/bin/zed` Preview wrapper unless the user explicitly asks to route terminal `zed` invocations back to Preview.
- Zed Stable and Preview reject unreleased wasm API versions. The `next` feature is only valid when you load the extension in Zed Nightly or a local Zed Dev build.
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
