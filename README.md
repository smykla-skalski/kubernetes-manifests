# Kubernetes

Kubernetes is a standalone Zed extension that adds a distinct `Kubernetes` language mode on top of the same Tree-sitter YAML grammar revision Zed uses for built-in YAML support, backed by `yaml-language-server`. The repo also ships an optional Helm language server integration and a Kubernetes context server for chat workflows.

The extension auto-detects Kubernetes in two ways: by Kubernetes-specific file suffixes (`*.k8s.yaml`, `*.kubernetes.yml`), and by best-effort content matching when the very first line starts with `apiVersion:`. Files opened with a generic `.yaml` extension are still claimed by Zed's built-in YAML language, which wins over content-based detection unless you explicitly remap those file types or switch the language manually.

Markdown fenced code blocks that use the `kubernetes` info string get Kubernetes syntax highlighting. Full Kubernetes LSP inside embedded regions is not available because Zed attaches language servers at the buffer level.

## Configuration

Use a `settings.json` file to configure these surfaces in every build. When the extension is built with `--features next` and loaded in Zed Nightly or a local Zed Dev build, the Kubernetes LSP settings gain typed schema-backed validation and completion under `lsp.kubernetes-language-server.*`.

The important boundary is that Zed does not surface these `next`-only LSP schema hooks on the extension card. The Extensions page `Configure` button is only for the context server. The `next` feature flag affects the Settings Editor and `settings.json` experience under `lsp.kubernetes-language-server.*` and `lsp.helm-language-server.*`.

- `lsp.kubernetes-language-server.*` for buffers that are actually in `Kubernetes` mode
- `lsp.yaml-language-server.*` for plain YAML buffers that stay in built-in `YAML` mode
- `lsp.helm-language-server.*` after you explicitly opt Helm into `languages.Kubernetes.language_servers`
- `context_servers.kubernetes-context-server.*` for the optional context server

The Kubernetes language server has two configuration layers under `lsp.kubernetes-language-server.settings`:

- `settings.kubernetes` is extension-owned. It covers the main YAML/Kubernetes editing experience: schema defaults and injection, schema store and CRD store, validation, completion, formatting, editor defaults, key ordering, custom tags, and extra schema associations. In `next` builds on Zed Nightly or Dev, this block gets typed schema-backed validation and completion in the LSP settings surface.
- `settings.yaml` is raw `yaml-language-server` workspace configuration that applies only to Kubernetes-mode buffers.

Precedence inside Kubernetes mode is `extension defaults < settings.kubernetes < settings.yaml`.

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
      "initialization_options": {
        "provideFormatter": true
      },
      "settings": {
        "kubernetes": {
          "includeDefaultSchemas": true,
          "injectIntoYamlLanguageServer": true,
          "validate": true,
          "completion": true,
          "schemaStore": {
            "enable": true
          },
          "kubernetesCRDStore": {
            "enable": true
          },
          "format": {
            "enable": true,
            "printWidth": 100
          },
          "editor": {
            "tabSize": 2
          },
          "schemaAssociations": {
            "./schemas/my-crd.json": ["crds/*.yaml"]
          }
        },
        "yaml": {
          "hover": true,
          "completion": true,
          "schemas": {
            "~/schemas/team-k8s.json": ["team/*.yaml"]
          }
        }
      }
    }
  }
}
```

Relative schema paths in both `settings.kubernetes.schemaAssociations` and `settings.yaml.schemas` resolve against the worktree root. `~/...` paths resolve against `HOME`. The extension normalizes those local schema paths to `file://` URLs before sending them to YAML LS, which avoids URI parsing failures from plain filesystem paths. Only the extension-owned schema associations are mirrored into built-in `yaml-language-server`; raw `settings.yaml` stays scoped to Kubernetes-mode buffers. Keep using raw `settings.yaml` for niche upstream options the curated layer does not expose yet, such as proxy settings or less-common YAML LS flags.

To force generic YAML files into Kubernetes mode, add `file_types` to your Zed settings:

```json
{
  "file_types": {
    "Kubernetes": ["*.yaml", "*.yml"]
  }
}
```

You can scope this to a project by putting it in `.zed/settings.json` at the project root instead of your global settings. This tells Zed to treat all matching files as Kubernetes, giving you Kubernetes-specific highlights, schema validation, and LSP support.

If you want to keep generic `.yaml` files on built-in `YAML`, configure them through `lsp.yaml-language-server` instead:

```json
{
  "lsp": {
    "yaml-language-server": {
      "settings": {
        "yaml": {
          "schemas": {
            "./schemas/platform.json": ["platform/*.yaml"],
            "~/schemas/global.json": ["global/*.yaml"]
          }
        }
      }
    }
  }
}
```

To enable Helm tooling inside `Kubernetes` mode, opt `helm-language-server` into the language server list and then configure it through its own raw `helm-ls` block:

```json
{
  "languages": {
    "Kubernetes": {
      "language_servers": ["helm-language-server", "..."]
    }
  },
  "lsp": {
    "helm-language-server": {
      "binary": {
        "path": "/opt/homebrew/bin/helm_ls",
        "arguments": ["serve"]
      },
      "settings": {
        "helm-ls": {
          "yamlls": {
            "enabled": false
          }
        }
      }
    }
  }
}
```

The Helm server stays opt-in because many Kubernetes users do not edit Helm templates in every project. The repo does not add a separate Helm language or template-language mode; the Helm integration is a second LSP attached to `Kubernetes` buffers when you opt in.

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

If you need the unreleased Zed extension API for typed Settings Editor hooks, build the wasm with the `next` feature instead:

```sh
mise run build:wasm --features next
```

If the isolated Zed profile is holding on to stale language-server state, rotate the runtime cache before relaunching:

```sh
mise run zed:sync-extension
```

Install the repo into the isolated validation profile as a dev extension without using the picker UI:

```sh
mise run zed:install-dev-extension
```

That is the right workflow for the released Zed extension API and for Zed Stable or Preview.

To test the unreleased `next` feature path, install the generated Nightly-only dev source tree instead of relying on Zed's stock "Install Dev Extension" compile path against the repo root:

```sh
mise run zed:install-next-extension
```

That task prepares `/tmp/zed-k8s-next-dev-source`, where plain `cargo build` defaults to the `next` feature, compiles the full extension payload through Zed's own builder so `extension.wasm` and `grammars/yaml.wasm` exist up front, and then installs that generated directory as a real dev extension symlink so it appears in the Extensions UI with `Rebuild` and `Configure`.

If you want to drive the same flow through Zed's picker UI, run:

```sh
mise run zed:prepare-next-dev-source
```

Then choose `/tmp/zed-k8s-next-dev-source` in `Install Dev Extension` instead of the repo root.

If you want the generic packaging-style local install for registry-like validation, use:

```sh
mise run zed:install-built-extension --features next
```

Use the `next` workflow only with Zed Nightly or a local Zed Dev build. Zed's host code explicitly rejects unreleased extension APIs on Stable and Preview, and the stock "Install Dev Extension" flow recompiles Rust extensions with plain `cargo build` and no custom feature flags. Also note that Zed's Installed tab only surfaces marketplace entries and true dev extensions; a manually copied local package can be loaded by the runtime but still remain invisible in the UI.

If you want one command that does the whole `next` flow, use:

```sh
mise run zed:next
mise run zed:next fixtures/valid/deployment.k8s.yaml
```

`zed:next` makes sure a real `Zed Nightly.app` is available first, restarts any already running Nightly instance, installs the generated `next` dev extension into the real Zed Nightly profile under `~/Library/Application Support/Zed`, clears repo-relative `CARGO_HOME` and `RUSTUP_HOME` before launch so Zed sees a sane rustup toolchain, and then launches `zed --nightly` against that profile.

Because that flow uses Nightly's real profile on macOS, worktree trust also comes from your real Nightly settings. The task no longer forces `ZED_STATELESS`, so if you manually trust the repo once, Nightly can persist that decision between runs. If the repo still opens in Restricted Mode, Zed will wait before starting `kubernetes-language-server` and `helm-language-server`. Trust the worktree in the UI, or add a Nightly-only override to `~/.config/zed/settings.json`:

```json
{
  "nightly": {
    "session": {
      "trust_all_worktrees": true
    }
  }
}
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

If you are testing the unreleased `next` feature path, install the generated dev-extension variant into the target profile instead and launch Zed Nightly or a local Zed Dev build against that same profile:

```sh
curl -f https://zed.dev/install.sh | ZED_CHANNEL=nightly sh
mise run zed:install-next-extension --user-data-dir "$HOME/Library/Application Support/Zed"
zed --nightly
```

Or use the single-task wrapper:

```sh
mise run zed:next
```

## Manual validation

Run this sequence after the automated checks are green. Record `pass`, `fail`, or `blocked` for every step, and include the exact blocker when a step cannot be completed.

Troubleshooting notes:

- Use `mise run zed:install-dev-extension` and the `mise run zed:foreground...` tasks instead of ad hoc Zed launches when you need clearer extension logs.
- Use `mise run zed:install-next-extension` plus Zed Nightly or a local Zed Dev build when you are testing the unreleased `next` feature flag. That task installs a real dev extension backed by `/tmp/zed-k8s-next-dev-source`, so the Extensions UI shows the entry and the `Rebuild` button keeps using plain `cargo build` against a Cargo manifest whose default feature is `next`.
- `mise run zed:next` is the fastest happy-path entrypoint for `next`: it bootstraps a real Zed Nightly install when needed, restarts any already running Nightly instance, rebuilds the extension, installs it as a dev extension into the real Nightly profile, scrubs repo-relative rustup and cargo env vars before launch, and launches Nightly in one command.
- On macOS, `zed --nightly` works by asking AppleScript for `path to application "Zed Nightly"` and then spawning that app bundle's CLI. If you still get `Could not determine app path for Zed Nightly`, make sure `Zed Nightly.app` is installed and run the command from a normal GUI terminal session.
- On macOS, Zed's background app-launch path ignores custom `--user-data-dir` values. That is why `zed:next` targets the real Nightly profile by default instead of pretending to isolate a separate `/tmp` profile through `zed --nightly`.
- Zed's Rust dev-extension builder probes `rustc --print sysroot` before it changes directories into the extension. Launch Nightly through `mise run zed:next` or clear repo-relative `CARGO_HOME` and `RUSTUP_HOME` yourself first, otherwise Zed can inherit `tmp/rustup` and `tmp/cargo` from `mise` and fail before the build even starts.
- The managed YAML LS fallback currently patches the installed `settingsHandlers.js` at runtime to remove an upstream unreleased `scopeUri: "null"` request that Zed rejects with `relative URL without a base`. If that log line comes back, first check whether a newer published `yaml-language-server` release has made the local patch obsolete.
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
9. Open `fixtures/valid/plain-deployment.yaml` and confirm the plain `.yaml` file stays on built-in `YAML` by default. If you enabled the `file_types` recipe above, confirm the same file opens as `Kubernetes` instead.
10. Open `fixtures/valid/plain-multi-document.yaml` and confirm the comment-prefixed multi-document file stays on built-in `YAML` unless you manually switch the buffer to `Kubernetes`.
11. Open `fixtures/invalid/plain-non-kubernetes.yaml` and confirm it stays on built-in `YAML`.
12. Open `fixtures/invalid/invalid-service.k8s.yaml` and confirm diagnostics are reported.
13. Open `fixtures/embedded/example.md` and confirm the fenced `kubernetes` code block gets Kubernetes syntax highlighting.
14. Open `fixtures/templates/deployment.tpl`, manually select the `Kubernetes` language for the whole buffer, and confirm the manual whole-buffer workflow behaves as expected for a template-oriented file.
15. Confirm the language-server status UI shows `Kubernetes Language Server`.
16. Select the bundled `Kubernetes` icon theme and confirm `*.k8s.yaml` or `*.kubernetes.yml` files use the Kubernetes icon in the language picker instead of the YAML fallback.
