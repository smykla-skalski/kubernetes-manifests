use zed_extension_api::{self as zed, KeyValueStore};

const DOCS_PROVIDER: &str = "kubernetes";

const KUBERNETES_DOCS_BASE: &str =
    "https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31";

const PACKAGES: &[(&str, &str)] = &[
    ("Deployment", "#deployment-v1-apps"),
    ("StatefulSet", "#statefulset-v1-apps"),
    ("DaemonSet", "#daemonset-v1-apps"),
    ("ReplicaSet", "#replicaset-v1-apps"),
    ("Job", "#job-v1-batch"),
    ("CronJob", "#cronjob-v1-batch"),
    ("Pod", "#pod-v1-core"),
    ("Service", "#service-v1-core"),
    ("Ingress", "#ingress-v1-networking-k8s-io"),
    ("ConfigMap", "#configmap-v1-core"),
    ("Secret", "#secret-v1-core"),
    ("PersistentVolumeClaim", "#persistentvolumeclaim-v1-core"),
    ("PersistentVolume", "#persistentvolume-v1-core"),
    ("StorageClass", "#storageclass-v1-storage-k8s-io"),
    ("Namespace", "#namespace-v1-core"),
    ("ServiceAccount", "#serviceaccount-v1-core"),
    ("Role", "#role-v1-rbac-authorization-k8s-io"),
    ("ClusterRole", "#clusterrole-v1-rbac-authorization-k8s-io"),
    ("RoleBinding", "#rolebinding-v1-rbac-authorization-k8s-io"),
    (
        "ClusterRoleBinding",
        "#clusterrolebinding-v1-rbac-authorization-k8s-io",
    ),
    (
        "HorizontalPodAutoscaler",
        "#horizontalpodautoscaler-v2-autoscaling",
    ),
    ("NetworkPolicy", "#networkpolicy-v1-networking-k8s-io"),
    ("Node", "#node-v1-core"),
    ("Endpoints", "#endpoints-v1-core"),
    ("Event", "#event-v1-events-k8s-io"),
    ("LimitRange", "#limitrange-v1-core"),
    ("ResourceQuota", "#resourcequota-v1-core"),
];

pub(crate) fn is_docs_provider(provider: &str) -> bool {
    provider == DOCS_PROVIDER
}

pub(crate) fn suggest_packages() -> Vec<String> {
    PACKAGES.iter().map(|(name, _)| name.to_string()).collect()
}

pub(crate) fn index_package(package: &str, database: &KeyValueStore) -> zed::Result<()> {
    let (_, anchor) = PACKAGES
        .iter()
        .find(|(name, _)| *name == package)
        .ok_or_else(|| format!("Unknown Kubernetes resource: {package}"))?;

    let url = format!("{KUBERNETES_DOCS_BASE}/{anchor}");

    let response = zed::http_client::fetch(&zed::http_client::HttpRequest {
        method: zed::http_client::HttpMethod::Get,
        url: url.clone(),
        headers: vec![("Accept".to_string(), "text/html".to_string())],
        body: None,
        redirect_policy: zed::http_client::RedirectPolicy::FollowAll,
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
        let html = (0..300)
            .map(|i| format!("<p>Line {i}</p>\n"))
            .collect::<String>();
        let text = extract_text_content(&html);
        let lines: Vec<_> = text.lines().collect();
        assert!(lines.len() <= 201);
    }
}
