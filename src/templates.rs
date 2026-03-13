const TEMPLATES: &[(&str, &str)] = &[
    ("ClusterRole", CLUSTER_ROLE),
    ("ClusterRoleBinding", CLUSTER_ROLE_BINDING),
    ("ConfigMap", CONFIGMAP),
    ("CronJob", CRONJOB),
    ("DaemonSet", DAEMONSET),
    ("Deployment", DEPLOYMENT),
    ("EndpointSlice", ENDPOINT_SLICE),
    ("HorizontalPodAutoscaler", HORIZONTAL_POD_AUTOSCALER),
    ("Ingress", INGRESS),
    ("Job", JOB),
    ("LimitRange", LIMIT_RANGE),
    ("Namespace", NAMESPACE),
    ("NetworkPolicy", NETWORK_POLICY),
    ("PersistentVolume", PERSISTENT_VOLUME),
    ("PersistentVolumeClaim", PERSISTENT_VOLUME_CLAIM),
    ("Pod", POD),
    ("PodDisruptionBudget", POD_DISRUPTION_BUDGET),
    ("ReplicaSet", REPLICA_SET),
    ("ResourceQuota", RESOURCE_QUOTA),
    ("Role", ROLE),
    ("RoleBinding", ROLE_BINDING),
    ("Secret", SECRET),
    ("Service", SERVICE),
    ("ServiceAccount", SERVICE_ACCOUNT),
    ("StatefulSet", STATEFULSET),
    ("StorageClass", STORAGE_CLASS),
];

pub fn resource_kinds() -> impl Iterator<Item = &'static str> {
    TEMPLATES.iter().map(|(kind, _)| *kind)
}

pub fn template_for_kind(kind: &str) -> Option<&'static str> {
    TEMPLATES
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, template)| *template)
}

const DEPLOYMENT: &str = "\
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-deployment
  labels:
    app: my-app
spec:
  replicas: 1
  selector:
    matchLabels:
      app: my-app
  template:
    metadata:
      labels:
        app: my-app
    spec:
      containers:
        - name: app
          image: nginx:latest
          ports:
            - containerPort: 80
";

const STATEFULSET: &str = "\
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: my-statefulset
spec:
  serviceName: my-statefulset
  replicas: 1
  selector:
    matchLabels:
      app: my-app
  template:
    metadata:
      labels:
        app: my-app
    spec:
      containers:
        - name: app
          image: nginx:latest
          ports:
            - containerPort: 80
";

const DAEMONSET: &str = "\
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: my-daemonset
spec:
  selector:
    matchLabels:
      app: my-app
  template:
    metadata:
      labels:
        app: my-app
    spec:
      containers:
        - name: app
          image: nginx:latest
";

const JOB: &str = "\
apiVersion: batch/v1
kind: Job
metadata:
  name: my-job
spec:
  template:
    spec:
      containers:
        - name: job
          image: busybox:latest
          command:
            - echo
            - hello
      restartPolicy: Never
  backoffLimit: 4
";

const CRONJOB: &str = "\
apiVersion: batch/v1
kind: CronJob
metadata:
  name: my-cronjob
spec:
  schedule: \"*/5 * * * *\"
  jobTemplate:
    spec:
      template:
        spec:
          containers:
            - name: job
              image: busybox:latest
              command:
                - echo
                - hello
          restartPolicy: Never
";

const SERVICE: &str = "\
apiVersion: v1
kind: Service
metadata:
  name: my-service
spec:
  selector:
    app: my-app
  ports:
    - port: 80
      targetPort: 80
  type: ClusterIP
";

const INGRESS: &str = "\
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: my-ingress
spec:
  rules:
    - host: example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: my-service
                port:
                  number: 80
";

const CONFIGMAP: &str = "\
apiVersion: v1
kind: ConfigMap
metadata:
  name: my-configmap
data:
  key: value
";

const SECRET: &str = "\
apiVersion: v1
kind: Secret
metadata:
  name: my-secret
type: Opaque
stringData:
  key: value
";

const PERSISTENT_VOLUME_CLAIM: &str = "\
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: my-pvc
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 1Gi
";

const SERVICE_ACCOUNT: &str = "\
apiVersion: v1
kind: ServiceAccount
metadata:
  name: my-service-account
";

const ROLE: &str = "\
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: my-role
rules:
  - apiGroups:
      - \"\"
    resources:
      - pods
    verbs:
      - get
      - list
      - watch
";

const CLUSTER_ROLE: &str = "\
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: my-clusterrole
rules:
  - apiGroups:
      - \"\"
    resources:
      - pods
    verbs:
      - get
      - list
      - watch
";

const ROLE_BINDING: &str = "\
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: my-rolebinding
subjects:
  - kind: ServiceAccount
    name: my-service-account
    namespace: default
roleRef:
  kind: Role
  name: my-role
  apiGroup: rbac.authorization.k8s.io
";

const CLUSTER_ROLE_BINDING: &str = "\
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: my-clusterrolebinding
subjects:
  - kind: ServiceAccount
    name: my-service-account
    namespace: default
roleRef:
  kind: ClusterRole
  name: my-clusterrole
  apiGroup: rbac.authorization.k8s.io
";

const NAMESPACE: &str = "\
apiVersion: v1
kind: Namespace
metadata:
  name: my-namespace
";

const POD: &str = "\
apiVersion: v1
kind: Pod
metadata:
  name: my-pod
spec:
  containers:
    - name: app
      image: nginx:latest
      ports:
        - containerPort: 80
";

const HORIZONTAL_POD_AUTOSCALER: &str = "\
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: my-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: my-deployment
  minReplicas: 1
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 80
";

const NETWORK_POLICY: &str = "\
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: my-network-policy
spec:
  podSelector:
    matchLabels:
      app: my-app
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - podSelector:
            matchLabels:
              app: frontend
      ports:
        - port: 80
";

const POD_DISRUPTION_BUDGET: &str = "\
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: my-pdb
spec:
  minAvailable: 1
  selector:
    matchLabels:
      app: my-app
";

const LIMIT_RANGE: &str = "\
apiVersion: v1
kind: LimitRange
metadata:
  name: my-limitrange
spec:
  limits:
    - type: Container
      default:
        cpu: 500m
        memory: 128Mi
      defaultRequest:
        cpu: 100m
        memory: 64Mi
";

const RESOURCE_QUOTA: &str = "\
apiVersion: v1
kind: ResourceQuota
metadata:
  name: my-quota
spec:
  hard:
    pods: \"10\"
    requests.cpu: \"4\"
    requests.memory: 8Gi
    limits.cpu: \"8\"
    limits.memory: 16Gi
";

const STORAGE_CLASS: &str = "\
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: my-storageclass
provisioner: kubernetes.io/no-provisioner
volumeBindingMode: WaitForFirstConsumer
";

const PERSISTENT_VOLUME: &str = "\
apiVersion: v1
kind: PersistentVolume
metadata:
  name: my-pv
spec:
  capacity:
    storage: 10Gi
  accessModes:
    - ReadWriteOnce
  persistentVolumeReclaimPolicy: Retain
  storageClassName: my-storageclass
  hostPath:
    path: /mnt/data
";

const REPLICA_SET: &str = "\
apiVersion: apps/v1
kind: ReplicaSet
metadata:
  name: my-replicaset
  labels:
    app: my-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: my-app
  template:
    metadata:
      labels:
        app: my-app
    spec:
      containers:
        - name: app
          image: nginx:latest
          ports:
            - containerPort: 80
";

const ENDPOINT_SLICE: &str = "\
apiVersion: discovery.k8s.io/v1
kind: EndpointSlice
metadata:
  name: my-endpointslice
  labels:
    kubernetes.io/service-name: my-service
addressType: IPv4
ports:
  - name: http
    port: 80
    protocol: TCP
endpoints:
  - addresses:
      - 10.0.0.1
    conditions:
      ready: true
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_for_unknown_kind_returns_none() {
        assert!(template_for_kind("UnknownKind").is_none());
    }

    #[test]
    fn all_templates_contain_their_kind() {
        for kind in resource_kinds() {
            let template = template_for_kind(kind).expect("template should exist");
            assert!(
                template.contains(&format!("kind: {kind}")),
                "template for {kind} should contain kind: {kind}",
            );
        }
    }
}
