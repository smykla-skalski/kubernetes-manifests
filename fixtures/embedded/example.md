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

The `k8s` info string also works:

```k8s
apiVersion: apps/v1
kind: Deployment
metadata:
  name: embedded-k8s-example
spec:
  replicas: 1
  selector:
    matchLabels:
      app: example
  template:
    metadata:
      labels:
        app: example
    spec:
      containers:
        - name: app
          image: nginx:latest
```
