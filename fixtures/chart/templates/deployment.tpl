apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "demo.fullname" . }}
  labels:
    app.kubernetes.io/name: {{ .Chart.Name | quote }}
spec:
  replicas: {{ default 2 .Values.replicas }}
  selector:
    matchLabels:
      app.kubernetes.io/name: {{ .Chart.Name | quote }}
  template:
    metadata:
      labels:
        app.kubernetes.io/name: {{ .Chart.Name | quote }}
    spec:
      containers:
        - name: web
          image: {{ printf "%s:%s" .Values.image.repository .Values.image.tag | quote }}
          ports:
            - containerPort: 80
