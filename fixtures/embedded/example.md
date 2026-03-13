# Embedded Example

This Markdown document should keep regular Markdown behavior while still syntax-highlighting fenced Kubernetes manifests.

```kubernetes
apiVersion: v1
kind: ConfigMap
metadata:
  name: embedded-example
data:
  app.yaml: |
    port: 8080
```
