use zed_extension_api::{
    self as zed,
    http_client::{fetch, HttpMethod, HttpRequest, RedirectPolicy},
    KeyValueStore,
};

const DOCS_PROVIDER: &str = "kubernetes";

const KUBERNETES_DOCS_BASE: &str =
    "https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.35";

const PACKAGES: &[(&str, &str)] = &[
    ("ClusterRole", "#clusterrole-v1-rbac-authorization-k8s-io"),
    (
        "ClusterRoleBinding",
        "#clusterrolebinding-v1-rbac-authorization-k8s-io",
    ),
    ("ConfigMap", "#configmap-v1-core"),
    ("CronJob", "#cronjob-v1-batch"),
    ("CSIDriver", "#csidriver-v1-storage-k8s-io"),
    (
        "CustomResourceDefinition",
        "#customresourcedefinition-v1-apiextensions-k8s-io",
    ),
    ("DaemonSet", "#daemonset-v1-apps"),
    ("Deployment", "#deployment-v1-apps"),
    ("Endpoints", "#endpoints-v1-core"),
    ("EndpointSlice", "#endpointslice-v1-discovery-k8s-io"),
    ("Event", "#event-v1-events-k8s-io"),
    (
        "HorizontalPodAutoscaler",
        "#horizontalpodautoscaler-v2-autoscaling",
    ),
    ("Ingress", "#ingress-v1-networking-k8s-io"),
    ("Job", "#job-v1-batch"),
    ("Lease", "#lease-v1-coordination-k8s-io"),
    ("LimitRange", "#limitrange-v1-core"),
    (
        "MutatingWebhookConfiguration",
        "#mutatingwebhookconfiguration-v1-admissionregistration-k8s-io",
    ),
    ("Namespace", "#namespace-v1-core"),
    ("NetworkPolicy", "#networkpolicy-v1-networking-k8s-io"),
    ("Node", "#node-v1-core"),
    ("PersistentVolume", "#persistentvolume-v1-core"),
    ("PersistentVolumeClaim", "#persistentvolumeclaim-v1-core"),
    ("Pod", "#pod-v1-core"),
    ("PodDisruptionBudget", "#poddisruptionbudget-v1-policy"),
    ("PriorityClass", "#priorityclass-v1-scheduling-k8s-io"),
    ("ReplicaSet", "#replicaset-v1-apps"),
    ("ResourceQuota", "#resourcequota-v1-core"),
    ("Role", "#role-v1-rbac-authorization-k8s-io"),
    ("RoleBinding", "#rolebinding-v1-rbac-authorization-k8s-io"),
    ("Secret", "#secret-v1-core"),
    ("Service", "#service-v1-core"),
    ("ServiceAccount", "#serviceaccount-v1-core"),
    ("StatefulSet", "#statefulset-v1-apps"),
    ("StorageClass", "#storageclass-v1-storage-k8s-io"),
    (
        "ValidatingWebhookConfiguration",
        "#validatingwebhookconfiguration-v1-admissionregistration-k8s-io",
    ),
    ("VolumeAttachment", "#volumeattachment-v1-storage-k8s-io"),
];

pub fn is_docs_provider(provider: &str) -> bool {
    provider == DOCS_PROVIDER
}

pub fn suggest_packages() -> Vec<String> {
    PACKAGES.iter().map(|(name, _)| name.to_string()).collect()
}

pub fn explain_resource(kind: &str) -> String {
    let Some((_, anchor)) = PACKAGES.iter().find(|(name, _)| *name == kind) else {
        return format!("# {kind}\n\nNo documentation available for this resource type.");
    };

    let url = format!("{KUBERNETES_DOCS_BASE}/{anchor}");

    let response = fetch(&HttpRequest {
        method: HttpMethod::Get,
        url: url.clone(),
        headers: vec![("Accept".to_string(), "text/html".to_string())],
        body: None,
        redirect_policy: RedirectPolicy::FollowAll,
    });

    let body = match response {
        Ok(response) => String::from_utf8_lossy(&response.body).into_owned(),
        Err(_) => {
            return format!(
                "# {kind}\n\nKubernetes API reference: {url}\n\n\
                 See the official documentation for field definitions and usage examples."
            );
        }
    };

    let content = extract_text_content(&body);

    if content.is_empty() {
        return format!(
            "# {kind}\n\nKubernetes API reference: {url}\n\n\
             See the official documentation for field definitions and usage examples."
        );
    }

    format!("# {kind}\n\nKubernetes API reference: {url}\n\n{content}")
}

pub fn index_package(package: &str, database: &KeyValueStore) -> zed::Result<()> {
    let (_, anchor) = PACKAGES
        .iter()
        .find(|(name, _)| *name == package)
        .ok_or_else(|| format!("Unknown Kubernetes resource: {package}"))?;

    let url = format!("{KUBERNETES_DOCS_BASE}/{anchor}");

    let response = fetch(&HttpRequest {
        method: HttpMethod::Get,
        url: url.clone(),
        headers: vec![("Accept".to_string(), "text/html".to_string())],
        body: None,
        redirect_policy: RedirectPolicy::FollowAll,
    });

    let body = match response {
        Ok(response) => String::from_utf8_lossy(&response.body).into_owned(),
        Err(_) => return index_fallback(package, anchor, database),
    };

    let content = extract_text_content(&body);

    if content.is_empty() {
        return index_fallback(package, anchor, database);
    }

    database.insert(
        &format!("{package}/overview"),
        &format!(
            "# {package}\n\n\
             Kubernetes API reference: {url}\n\n\
             {content}"
        ),
    )?;

    Ok(())
}

fn index_fallback(package: &str, anchor: &str, database: &KeyValueStore) -> zed::Result<()> {
    let url = format!("{KUBERNETES_DOCS_BASE}/{anchor}");
    database.insert(
        &format!("{package}/overview"),
        &format!(
            "# {package}\n\n\
             Kubernetes API reference: {url}\n\n\
             See the official documentation for field definitions \
             and usage examples."
        ),
    )?;
    Ok(())
}

fn extract_text_content(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut line_count = 0;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '\n' if !in_tag => {
                if !result.ends_with('\n') {
                    result.push('\n');
                    line_count += 1;
                }
                if line_count > 200 {
                    break;
                }
            }
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docs_provider_matches_kubernetes() {
        assert!(is_docs_provider("kubernetes"));
        assert!(!is_docs_provider("other"));
    }

    #[test]
    fn suggest_packages_returns_all_resources() {
        let packages = suggest_packages();
        assert!(packages.contains(&"Deployment".to_string()));
        assert!(packages.contains(&"Service".to_string()));
        assert!(packages.contains(&"ConfigMap".to_string()));
        assert_eq!(packages.len(), PACKAGES.len());
    }

    #[test]
    fn extract_text_content_strips_html_tags() {
        let html = "<h1>Title</h1><p>Some <b>bold</b> text</p>";
        let text = extract_text_content(html);
        assert_eq!(text, "TitleSome bold text");
    }

    #[test]
    fn extract_text_content_limits_output_lines() {
        use std::fmt::Write;
        let html = (0..300).fold(String::new(), |mut acc, i| {
            writeln!(acc, "<p>Line {i}</p>").unwrap();
            acc
        });
        let text = extract_text_content(&html);
        assert!(text.lines().count() <= 201);
    }
}
