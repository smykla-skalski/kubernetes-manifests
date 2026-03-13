# Repository Guidelines

## Project Structure & Module Organization

`src/` holds the Rust extension code. Keep [src/kubernetes.rs](src/kubernetes.rs) as the thin entrypoint, [src/language_server.rs](src/language_server.rs) for `yaml-language-server` resolution and command assembly, and [src/settings.rs](src/settings.rs) for pure configuration merge logic. [languages/kubernetes/](languages/kubernetes/) contains language config and query files. [fixtures/valid](fixtures/valid), [fixtures/invalid](fixtures/invalid), [fixtures/embedded](fixtures/embedded), and [fixtures/templates](fixtures/templates) drive manual validation. [extension.toml](extension.toml) defines the published Zed extension, while [.mise/tasks/](.mise/tasks/) and `.mise/*.swift` contain local automation and macOS Accessibility helpers.

## Build, Test, and Development Commands

Run `mise trust .mise.toml` once after cloning. Use `mise run test` for Rust tests (`cargo test` plus `cargo nextest`). Use `mise run lint` for `cargo fmt --check`, `cargo clippy`, Tree-sitter query formatting, and strict Swift formatting. Use `mise run check` as the default full local gate. Use `mise run build:wasm` to rebuild `extension.wasm`, `mise run package` to verify packaging with `zed-extension`, and `mise run zed:install-dev-extension` to install the repo into the isolated Zed profile before the manual validation checklist in `README.md`.

## Coding Style & Naming Conventions

Follow `rustfmt` and keep Clippy clean with `-D warnings`. Prefer small, explicit helpers over clever branching. Avoid `unwrap()` in production code. Plain `.yaml` detection is allowed here when the opening document block clearly looks like a Kubernetes manifest, so keep detection rules explicit and test-backed. Swift helper files under `.mise/` use `.swift-format` with 2-space indentation and strict linting. Use descriptive task and fixture names such as `deployment.k8s.yaml` or `zed:install-dev-extension`.

## Testing Guidelines

Add unit tests alongside the Rust modules they cover and use descriptive names such as `managed_node_command_places_script_path_before_server_arguments`. Treat `mise run check` and `mise run package` as required before review. If behavior changes in Zed itself, rerun the relevant fixture flow from the manual validation section in `README.md` and record exact pass/fail results.

## Commit & Pull Request Guidelines

This branch currently has no local commit history to infer conventions from, so use the repo house style directly: Conventional Commits with a scope, for example `feat(extension): tighten kubernetes schema defaults`, and create commits with `git commit -sS`. Pull requests should explain the user-visible change, list the validation commands you ran, and include manual Zed validation notes when editor behavior changed. Add screenshots only when UI or macOS automation behavior is part of the change.
