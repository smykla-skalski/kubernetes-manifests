# Kubernetes

Kubernetes is a standalone Zed extension that adds a distinct `Kubernetes` language mode on top of the same Tree-sitter YAML grammar revision Zed uses for built-in YAML support, plus `yaml-language-server`.

The extension auto-detects Kubernetes in two ways: by Kubernetes-specific file names such as `*.k8s.yaml` and `*.kubernetes.yml`, and by plain `.yaml` or `.yml` document headers when the opening document block contains both top-level `apiVersion:` and `kind:` keys. That detection is intentionally front-loaded to the opening content window Zed exposes to extensions, so files that only mention those keys much later in the buffer can still stay on built-in YAML.

Markdown fenced code blocks that use the `kubernetes` info string get Kubernetes syntax highlighting. Full Kubernetes LSP inside arbitrary embedded regions is not available from a standalone extension because Zed attaches language servers at the buffer level.

Use the Settings Editor or project `settings.json` to configure the language server id `kubernetes-language-server`. The extension merges your `lsp.kubernetes-language-server.settings` on top of its default Kubernetes schema association.

By default, the bundled schema association still targets Kubernetes-specific suffixes such as `*.k8s.yaml` and `*.kubernetes.yml`. If you want schema validation on non-standard template file names, add your own `lsp.kubernetes-language-server.settings.yaml.schemas` override.

```json
{
  "lsp": {
    "kubernetes-language-server": {
      "binary": {
        "path": "/opt/homebrew/bin/yaml-language-server",
        "arguments": ["--stdio"],
        "env": {
          "YAML_SCHEMA_STORE_ENABLE": "false"
        }
      },
      "settings": {
        "yaml": {
          "schemas": {
            "https://example.com/custom-schema.json": ["*.k8s.yaml"]
          }
        }
      }
    }
  }
}
```

The repo also ships a small optional icon theme overlay at `icon_themes/kubernetes.json`. Select the `Kubernetes` icon theme in Zed if you want Kubernetes-specific file-name matches and the language picker to use the bundled Kubernetes icon instead of YAML's default icon. Generic plain `.yaml` file icons still follow the active file icon theme.

## Local development

This repo uses `mise` for the normal build, validation, and local Zed workflow. Task files live under `.mise/tasks`; `.mise.toml` only tells mise to load machine-local environment values from `.env.local`.

Trust the project config once:

```sh
mise trust .mise.toml
```

List them with:

```sh
mise tasks ls
```

Run the full local gate:

```sh
mise run check
```

Use the smaller aggregate lanes when you want a quicker signal:

```sh
mise run test
mise run lint
```

`mise run test` now includes the wasm dev-extension build, so crate-name and artifact-path drift gets caught before `Install Dev Extension` breaks.

The validation split follows Zed's own extension workflow: Rust checks on one side, packaging and query checks on the other. Editor behavior is still a manual fixture pass. That is normal for Zed extensions.

After Rust changes, rebuild the checked-in WebAssembly artifact:

```sh
mise run build:wasm
```

If the isolated Zed profile is holding on to stale language-server state, rotate the runtime cache before relaunching:

```sh
mise run zed:sync-extension
```

Install the repo into the isolated validation profile as a dev extension without using the picker UI:

```sh
mise run zed:install-dev-extension
```

If you want a clean validation window without reinstalling the dev extension, seed a fresh profile from the default validation profile:

```sh
mise run zed:seed-profile
mise run zed:open --user-data-dir /tmp/zed-k8s-validation-clean fixtures/valid/deployment.k8s.yaml
```

Run the individual checks when you need a narrower signal:

```sh
mise run check:test
mise run check:wasm
mise run check:fmt
mise run check:clippy
mise run check:nextest
mise run check:queries
mise run lint:swift
```

Open the standard fixtures in an isolated Zed profile:

```sh
mise run zed:deployment
mise run zed:multi-document
mise run zed:invalid
mise run zed:embedded
mise run zed:template
mise run zed:plain
```

Or open any path in that same profile:

```sh
mise run zed:open fixtures/valid/deployment.k8s.yaml
mise run zed:open --user-data-dir /tmp/zed-k8s-alt fixtures/valid/deployment.k8s.yaml
mise run zed:open:new fixtures/valid/deployment.k8s.yaml
```

For live app logs while debugging Zed behavior, use the foreground task:

```sh
mise run zed:deployment:foreground
mise run zed:foreground fixtures/valid/deployment.k8s.yaml
mise run zed:foreground --user-data-dir /tmp/zed-k8s-alt fixtures/valid/deployment.k8s.yaml
mise run zed:foreground:new fixtures/valid/deployment.k8s.yaml
```

If you change Rust extension code or the managed language-server bootstrap path, rebuild with `mise run build:wasm`, rotate the isolated runtime with `mise run zed:sync-extension` if needed, and relaunch Zed before trusting the result.

For native macOS validation, use the file-backed `mise` helpers under `.mise/tasks/zed/...` instead of ad hoc `osascript` calls:

```sh
mise run zed:cg:count
mise run zed:cg:windows
mise run zed:ax:ensure-window
mise run zed:ax:new-window
mise run zed:ax:front-window
mise run zed:ax:windows
mise run zed:ax:buttons
mise run zed:ax:names --title-contains "kubernetes-manifests" --contains "Extensions"
mise run zed:ax:keystroke --title-contains "kubernetes-manifests" --key x --modifiers command,shift
mise run zed:ax:type --title-contains "kubernetes-manifests" --text "zed: install dev extension"
mise run zed:ax:key-code --title-contains "kubernetes-manifests" --key-code 36
mise run zed:ax:menu-bar-items
mise run zed:ax:menu-item --menu-item "Extensions"
mise run zed:ax:open-extensions
```

These `zed:ax:*` tasks need macOS Accessibility permission for the terminal process that runs `mise`, and they default to the `zed` process name exposed by Zed Preview on macOS.

The Space boundary is narrower than it first looked. The native AX tasks can still resolve Zed and drive menu actions when the app is parked in another Mission Control Space, including off-space `zed:ax:new-window` control. Use `mise run zed:cg:windows` for the broader CoreGraphics inventory across Spaces, and use `mise run zed:ax:windows` for the AX-targetable window that the other `zed:ax:*` tasks act on.

Prefer `zed:ax:menu-item`, `zed:ax:open-extensions`, `zed:ax:new-window`, and `zed:ax:ensure-window` when you need reliable off-space control. `zed:ax:keystroke`, `zed:ax:type`, and `zed:ax:key-code` post keyboard events to the Zed pid, so they still depend on whatever control Zed currently has focused.

These tasks accept flags through `mise`'s `usage` interface, so you can target a different process or window without editing the task file:

```sh
mise run zed:ax:buttons --window-index 2
mise run zed:ax:buttons --title-contains "Extensions"
mise run zed:ax:names --window-index 3 --contains "Install"
mise run zed:ax:menu-bar-items
mise run zed:ax:menu-item --app-menu "Zed" --menu-item "Extensions"
mise run zed:ax:menu-item --app-menu "" --app-menu-index 1 --menu-item "Extensions"
```

Check the generated help for any of these tasks with:

```sh
mise run zed:ax:buttons --help
mise run zed:ax:keystroke --help
```

These `zed:*` tasks prepare the isolated profile automatically and write a profile-local `settings.json` with `session.trust_all_worktrees = true`, so language servers are not blocked behind the trust prompt during manual validation.

`mise run zed:sync-extension` rebuilds `extension.wasm` and moves the isolated profile's `extensions/work/kubernetes` directory aside so the next launch reinstalls the managed language server instead of reusing stale runtime state.

The packaging and query-format tasks bootstrap their helper CLIs into `tmp/tools` automatically, so you do not need separate installs for `zed-extension` or `ts_query_ls`. On Linux x86_64, `zed-extension` is downloaded from Zed's published blob-store binary. On macOS, the task falls back to building `extension_cli` from `ZED_REPO` because Zed does not currently publish a Darwin prebuilt at that URL; that source build also requires the Metal toolchain to be available to `xcrun`.

The underlying Rust commands are still available directly:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --workspace --all-targets --no-fail-fast
```

Package the extension with:

```sh
mise run package
mise run check:queries
```

Install it in Zed with `mise run zed:install-dev-extension`, or use "Extensions: Install Dev Extension" and point Zed at this repository root.

If your normal Zed profile drifted from the repo's current extension id or runtime layout, repair that profile directly with:

```sh
mise run zed:install-dev-extension --user-data-dir "$HOME/Library/Application Support/Zed"
```

## Manual validation

Run this sequence after the automated checks are green. Record `pass`, `fail`, or `blocked` for every step, and include the exact blocker when a step cannot be completed.

Troubleshooting notes:

- Use `mise run zed:install-dev-extension` and the `mise run zed:foreground...` tasks instead of ad hoc Zed launches when you need clearer extension logs.
- After Rust changes or managed LSP bootstrap changes, rerun `mise run build:wasm`, then restart Zed or run `mise run zed:sync-extension` before reopening fixtures.
- Treat the fixtures as an expectation matrix: `fixtures/valid/*` should stay free of unexpected diagnostics, `fixtures/invalid/*` should report diagnostics, `fixtures/embedded/*` is syntax-highlighting-only, and `fixtures/templates/*` exercises the manual whole-buffer language-selection path.

1. Run `mise run test`.
2. Run `mise run lint`.
3. Run `mise run package`.
4. Run `mise run build:wasm`.
5. Run `mise run zed:install-dev-extension`.
6. If the isolated validation profile already has this dev extension installed, run `mise run zed:sync-extension` before reopening Zed.
7. Open `fixtures/valid/deployment.k8s.yaml` and confirm Zed auto-detects the `Kubernetes` language.
8. In `fixtures/valid/deployment.k8s.yaml`, confirm hover, completion, outline, formatting, and diagnostics all behave correctly.
9. Open `fixtures/valid/plain-deployment.yaml` and confirm the plain `.yaml` file auto-detects as `Kubernetes` from its `apiVersion` and `kind` header.
10. Open `fixtures/valid/plain-multi-document.yaml` and confirm the same header-based Kubernetes detection still works for multiple YAML documents.
11. Open `fixtures/invalid/plain-non-kubernetes.yaml` and confirm it stays on built-in YAML.
12. Open `fixtures/invalid/invalid-service.k8s.yaml` and confirm diagnostics are reported.
13. Open `fixtures/embedded/example.md` and confirm the fenced `kubernetes` code block gets Kubernetes syntax highlighting.
14. Open `fixtures/templates/deployment.tpl`, manually select the `Kubernetes` language for the whole buffer, and confirm the manual whole-buffer workflow behaves as expected for a template-oriented file.
15. Confirm the language-server status UI shows `Kubernetes Language Server`.
16. Select the bundled `Kubernetes` icon theme and confirm `*.k8s.yaml` or `*.kubernetes.yml` files use the Kubernetes icon in the language picker instead of the YAML fallback.
