apiVersion: apps/v1
kind: Deployment
metadata:
  name: "{{ include \"demo.fullname\" . }}"
  labels:
    app.kubernetes.io/name: "{{ .Chart.Name }}"
spec:
  replicas: {{ default 2 .Values.replicas }}
  selector:
    matchLabels:
      app.kubernetes.io/name: "{{ .Chart.Name }}"
  template:
    metadata:
      labels:
        app.kubernetes.io/name: "{{ .Chart.Name }}"
    spec:
      containers:
        - name: web
          image: "{{ .Values.image.repository }}:{{ .Values.image.tag }}"
          ports:
            - containerPort: 80
