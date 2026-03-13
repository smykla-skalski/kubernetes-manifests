# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A Zed editor extension that adds a `Kubernetes` language mode on top of Tree-sitter YAML, backed by `yaml-language-server`. It auto-detects Kubernetes manifests by file suffix (`*.k8s.yaml`, `*.kubernetes.yml`) and by scanning the opening lines for `apiVersion:` + `kind:`. Ships an optional icon theme overlay.

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

Three Rust source files compile to a `cdylib` WebAssembly extension:

- `src/kubernetes.rs` - thin extension entrypoint, implements `zed::Extension` trait, routes to language server and settings modules. Integration tests for `extension.toml`, `first_line_pattern` detection, and icon theme live here.
- `src/language_server.rs` - resolves `yaml-language-server` binary (user-configured path > `$PATH` lookup > managed npm install). Builds the `zed::Command` with merged args and env.
- `src/settings.rs` - produces default workspace configuration (Kubernetes schema globs, YAML formatting) and recursively merges user settings on top.

Language definition lives in `languages/kubernetes/config.toml` with Tree-sitter query files (`highlights.scm`, `outline.scm`, etc.) alongside it. The `first_line_pattern` regex does the plain-YAML Kubernetes auto-detection within a 25-line content window.

`extension.toml` declares the extension metadata, grammar source, npm capability for `yaml-language-server`, and the language server ID `kubernetes-language-server`.

## Key terms

- **content window** - the first ~25 lines of a buffer that Zed exposes to `first_line_pattern` for language detection (`languages/kubernetes/config.toml:5`)
- **worktree** - Zed's representation of an open project directory, passed to extension trait methods for per-project settings
- **schema globs** - the file patterns in `settings.rs` that tell `yaml-language-server` which files get Kubernetes schema validation
- **managed npm install** - the fallback path in `language_server.rs` where the extension installs `yaml-language-server` via `zed::npm_install_package` when no local binary is found

## Gotchas

- After any Rust change, you must run `mise run build:wasm` to rebuild `extension.wasm` before testing in Zed - the checked-in wasm is the artifact Zed loads, not a cargo build output
- `.mise.toml` sets `CARGO_HOME=tmp/cargo` and `RUSTUP_HOME=tmp/rustup`, so toolchains and crates live inside the project tree. Direct `cargo` invocations outside `mise` use your global toolchain instead
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
