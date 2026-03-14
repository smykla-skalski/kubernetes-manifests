# Kubernetes

A Zed extension that adds `Kubernetes` and `Helm` language modes with full LSP support, schema validation, and a context server for chat workflows.

## Detection

Kubernetes mode activates for `*.k8s.yaml`, `*.kubernetes.yml`, and `*.kyaml` files. Plain `.yaml` files that start with `apiVersion:` on the first line also match. Generic `.yaml` files stay on built-in YAML unless you remap them with `file_types`.

Helm mode activates for `.tpl` files and files starting with `{{` or `{{-`.

Markdown fenced code blocks with the `kubernetes` or `k8s` info string get syntax highlighting, but not LSP (Zed attaches language servers at the buffer level).

## Configuration

Four settings surfaces, each scoped to its own language mode:

- `lsp.kubernetes-language-server.*` - Kubernetes-mode buffers
- `lsp.yaml-language-server.*` - plain YAML buffers (built-in YAML mode)
- `lsp.helm-language-server.*` - Helm-mode buffers
- `context_servers.kubernetes-context-server.*` - context server

### Kubernetes language server

Two layers under `lsp.kubernetes-language-server.settings`:

- `settings.kubernetes` - extension-owned config for schemas, validation, completion, formatting, editor defaults, CRD store, custom tags, key ordering
- `settings.yaml` - raw `yaml-language-server` passthrough for Kubernetes-mode buffers only

Precedence: `extension defaults < settings.kubernetes < settings.yaml`.

```json
{
  "lsp": {
    "kubernetes-language-server": {
      "binary": {
        "path": "/opt/homebrew/bin/yaml-language-server",
        "arguments": ["--stdio"]
      },
      "settings": {
        "kubernetes": {
          "includeDefaultSchemas": true,
          "injectIntoYamlLanguageServer": true,
          "validate": true,
          "completion": true,
          "schemaStore": { "enable": true },
          "kubernetesCRDStore": { "enable": true },
          "format": { "enable": true, "printWidth": 100 },
          "editor": { "tabSize": 2 },
          "schemaAssociations": {
            "./schemas/my-crd.json": ["crds/*.yaml"]
          }
        },
        "yaml": {
          "hover": true,
          "schemas": {
            "~/schemas/team-k8s.json": ["team/*.yaml"]
          }
        }
      }
    }
  }
}
```

Relative schema paths resolve against the worktree root. `~/` paths resolve against `HOME`. The extension converts local paths to `file://` URLs before they reach YAML LS. Only extension-owned schema associations get mirrored into built-in `yaml-language-server`. Raw `settings.yaml` stays scoped to Kubernetes-mode buffers.

### Forcing all YAML into Kubernetes mode

```json
{
  "file_types": {
    "Kubernetes": ["*.yaml", "*.yml"]
  }
}
```

Scope this per-project by putting it in `.zed/settings.json`.

### Helm language server

`.tpl` files auto-detect as Helm. For `.yaml` files inside `templates/` directories, add a `file_types` mapping:

```json
{
  "file_types": {
    "Helm": ["**/templates/**/*.yaml", "**/templates/**/*.yml"]
  }
}
```

The community Helm extension (cabrinha/helm.zed) conflicts with this one. Uninstall it first.

```json
{
  "lsp": {
    "helm-language-server": {
      "binary": {
        "path": "/opt/homebrew/bin/helm_ls",
        "arguments": ["serve"]
      },
      "settings": {
        "helm-ls": {
          "yamlls": { "enabled": false }
        }
      }
    }
  }
}
```

### Icon theme

The extension ships an optional icon theme overlay. Select `Kubernetes` in Zed's icon theme picker to get Kubernetes-specific icons for `*.k8s.yaml`, `*.kubernetes.yml`, `*.kyaml`, and `*.tpl` files.

## Local development

Uses `mise` for build automation. Trust the config once with `mise trust .mise.toml`, then list all tasks with `mise tasks ls`.

```sh
mise run check          # full gate: fmt, clippy, nextest, queries, wasm, packaging
mise run test           # cargo test + nextest + wasm build
mise run lint           # fmt --check, clippy, query formatting, swift formatting
mise run package        # zed-extension CLI validation
mise run build:wasm     # rebuild extension.wasm after Rust changes
```

Install into an isolated Zed validation profile:

```sh
mise run zed:install-dev-extension
```

For the unreleased `next` API (Zed Nightly or Dev builds only):

```sh
mise run zed:next                           # single command: build, install, launch Nightly
mise run zed:next fixtures/valid/deployment.k8s.yaml
```

If stale language-server state persists across rebuilds:

```sh
mise run zed:sync-extension
```

Open fixtures in the isolated profile:

```sh
mise run zed:deployment
mise run zed:template
mise run zed:embedded
mise run zed:invalid
mise run zed:plain
```

For live Zed app logs, use the foreground variants:

```sh
mise run zed:foreground fixtures/valid/deployment.k8s.yaml
```

## Manual validation

Run after `mise run check && mise run package` passes. After any Rust change, run `mise run build:wasm` first.

Fixtures are an expectation matrix: `fixtures/valid/*` should be clean, `fixtures/invalid/*` should report diagnostics, `fixtures/embedded/*` is syntax-only, `fixtures/chart/templates/*` exercises Helm mode.

1. `mise run zed:install-dev-extension` (run `zed:sync-extension` first if re-installing)
2. Open `fixtures/valid/deployment.k8s.yaml` - confirm auto-detection as `Kubernetes`, hover, completion, outline, formatting, diagnostics
3. Open `fixtures/valid/plain-deployment.yaml` - confirm it stays on built-in `YAML`
4. Open `fixtures/valid/plain-multi-document.yaml` - confirm it stays on built-in `YAML`
5. Open `fixtures/invalid/plain-non-kubernetes.yaml` - confirm it stays on built-in `YAML`
6. Open `fixtures/invalid/invalid-service.k8s.yaml` - confirm diagnostics reported
7. Open `fixtures/embedded/example.md` - confirm both `kubernetes` and `k8s` fenced blocks get syntax highlighting
8. Open `fixtures/chart/templates/deployment.tpl` - confirm `Helm` detection, Go template highlights, YAML injection highlights, no false schema errors
9. Confirm helm-language-server starts, `.Values.` completions work from `fixtures/chart/values.yaml`
10. Open `fixtures/chart/templates/helpers.tpl` - confirm Helm auto-detection via first_line_pattern
11. Open `fixtures/valid/deployment.kyaml` - confirm `Kubernetes` detection, flow-style highlights, outline panel
12. Open `fixtures/valid/secret.kyaml` - confirm sensitive values are redacted
13. Confirm language-server status shows `Kubernetes Language Server`
14. Select `Kubernetes` icon theme, confirm correct icons on `*.k8s.yaml`, `*.kubernetes.yml`, `*.kyaml`, `*.tpl`
