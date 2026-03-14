use zed_extension_api::{
    self as zed, KeyValueStore,
    http_client::{HttpMethod, HttpRequest, RedirectPolicy, fetch},
};

const DOCS_PROVIDER: &str = "kubernetes";

/// Kubernetes minor version for API reference links.
/// Bump when a new Kubernetes release ships updated generated docs.
const KUBERNETES_DOCS_VERSION: &str = "v1.35";

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

fn package_anchor(kind: &str) -> Option<&'static str> {
    PACKAGES
        .iter()
        .find(|(name, _)| *name == kind)
        .map(|(_, anchor)| *anchor)
}

fn kubernetes_docs_url(anchor: &str) -> String {
    format!(
        "https://kubernetes.io/docs/reference/generated/kubernetes-api/{KUBERNETES_DOCS_VERSION}/{anchor}"
    )
}

pub fn explain_resource(kind: &str) -> String {
    let Some(anchor) = package_anchor(kind) else {
        return format!("# {kind}\n\nNo documentation available for this resource type.");
    };

    let url = kubernetes_docs_url(anchor);

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

    let content = extract_text_content(seek_to_anchor(&body, &url));

    if content.is_empty() {
        return format!(
            "# {kind}\n\nKubernetes API reference: {url}\n\n\
             See the official documentation for field definitions and usage examples."
        );
    }

    format!("# {kind}\n\nKubernetes API reference: {url}\n\n{content}")
}

pub fn index_package(package: &str, database: &KeyValueStore) -> zed::Result<()> {
    let anchor =
        package_anchor(package).ok_or_else(|| format!("Unknown Kubernetes resource: {package}"))?;

    let url = kubernetes_docs_url(anchor);

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

    let content = extract_text_content(seek_to_anchor(&body, &url));

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
    let url = kubernetes_docs_url(anchor);
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
    let mut tag_name = String::new();
    let mut in_tag = false;
    let mut collecting_tag_name = false;
    let mut is_closing_tag = false;
    let mut in_preformatted = false;
    let mut skip_content = false;
    let mut line_count = 0;

    for ch in html.chars() {
        if line_count > 200 {
            break;
        }

        match ch {
            '<' => {
                in_tag = true;
                collecting_tag_name = true;
                is_closing_tag = false;
                tag_name.clear();
            }
            '/' if in_tag && collecting_tag_name && tag_name.is_empty() => {
                is_closing_tag = true;
            }
            '>' => {
                in_tag = false;
                collecting_tag_name = false;
                let tag = tag_name.to_ascii_lowercase();

                match tag.as_str() {
                    "br" => {
                        push_newline(&mut result, &mut line_count);
                    }
                    "hr" => {
                        push_newline(&mut result, &mut line_count);
                        result.push_str("---");
                        push_newline(&mut result, &mut line_count);
                    }
                    "li" if !is_closing_tag => {
                        push_newline(&mut result, &mut line_count);
                        result.push_str("- ");
                    }
                    "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" if is_closing_tag => {
                        push_newline(&mut result, &mut line_count);
                    }
                    "pre" | "code" => {
                        in_preformatted = !is_closing_tag;
                    }
                    "script" | "style" => {
                        skip_content = !is_closing_tag;
                    }
                    _ => {}
                }
            }
            ' ' | '\t' if in_tag && collecting_tag_name => {
                collecting_tag_name = false;
            }
            _ if in_tag && collecting_tag_name => {
                tag_name.push(ch);
            }
            _ if in_tag || skip_content => {}
            '\n' if in_preformatted => {
                result.push('\n');
                line_count += 1;
            }
            '\n' => {
                push_newline(&mut result, &mut line_count);
            }
            _ => result.push(ch),
        }
    }

    decode_html_entities(result.trim())
}

fn decode_html_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn seek_to_anchor<'a>(html: &'a str, url: &str) -> &'a str {
    let anchor = match url.rsplit_once('#') {
        Some((_, fragment)) if !fragment.is_empty() => fragment,
        _ => return html,
    };
    for pattern in [format!("id=\"{anchor}\""), format!("id='{anchor}'")] {
        if let Some(pos) = html.find(pattern.as_str()) {
            let tag_start = html[..pos].rfind('<').unwrap_or(pos);
            return &html[tag_start..];
        }
    }
    html
}

fn push_newline(result: &mut String, line_count: &mut usize) {
    if !result.ends_with('\n') {
        result.push('\n');
        *line_count += 1;
    }
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
    fn seek_to_anchor_finds_id_with_double_quotes() {
        let html = r#"<div>preamble</div><h2 id="deployment-v1-apps">Deployment</h2><p>body</p>"#;
        let result = seek_to_anchor(html, "https://example.com/api#deployment-v1-apps");
        assert!(
            result.starts_with("<h2"),
            "should start from the tag containing the anchor id",
        );
        assert!(result.contains("Deployment"));
    }

    #[test]
    fn seek_to_anchor_finds_id_with_single_quotes() {
        let html = "<div>preamble</div><h2 id='my-anchor'>Title</h2>";
        let result = seek_to_anchor(html, "https://example.com/#my-anchor");
        assert!(result.starts_with("<h2"));
    }

    #[test]
    fn seek_to_anchor_returns_full_html_without_fragment() {
        let html = "<div>content</div>";
        let result = seek_to_anchor(html, "https://example.com/api");
        assert_eq!(result, html);
    }

    #[test]
    fn seek_to_anchor_returns_full_html_when_anchor_not_found() {
        let html = "<div>content</div>";
        let result = seek_to_anchor(html, "https://example.com/#nonexistent");
        assert_eq!(result, html);
    }

    #[test]
    fn seek_to_anchor_returns_full_html_with_empty_fragment() {
        let html = "<div>content</div>";
        let result = seek_to_anchor(html, "https://example.com/#");
        assert_eq!(result, html);
    }

    #[test]
    fn extract_text_content_strips_html_tags() {
        let html = "<h1>Title</h1><p>Some <b>bold</b> text</p>";
        let text = extract_text_content(html);
        assert_eq!(text, "Title\nSome bold text");
    }

    #[test]
    fn extract_text_content_preserves_list_items() {
        let html = "<ul><li>first</li><li>second</li><li>third</li></ul>";
        let text = extract_text_content(html);
        assert!(
            text.contains("- first"),
            "list items should be prefixed with dash"
        );
        assert!(text.contains("- second"));
        assert!(text.contains("- third"));
    }

    #[test]
    fn extract_text_content_preserves_preformatted_blocks() {
        let html = "<pre><code>line1\nline2\n  indented</code></pre>";
        let text = extract_text_content(html);
        assert!(
            text.contains("line1\nline2\n  indented"),
            "preformatted blocks should preserve whitespace literally",
        );
    }

    #[test]
    fn extract_text_content_inserts_paragraph_breaks() {
        let html = "<p>First paragraph.</p><p>Second paragraph.</p>";
        let text = extract_text_content(html);
        assert!(
            text.contains("First paragraph.\nSecond paragraph."),
            "closing </p> should emit a newline between paragraphs",
        );
    }

    #[test]
    fn extract_text_content_skips_script_content() {
        let html = "<p>Hello</p><script>var x = 1;</script><p>World</p>";
        let text = extract_text_content(html);
        assert!(!text.contains("var x"), "script content should be skipped");
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
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

    #[test]
    fn extract_text_content_decodes_html_entities() {
        let html = "<p>defaults &amp; optional &lt;fields&gt;</p>";
        let text = extract_text_content(html);
        assert_eq!(text, "defaults & optional <fields>");
    }

    #[test]
    fn extract_text_content_decodes_entities_in_preformatted_blocks() {
        let html = "<pre><code>if x &lt; 10 &amp;&amp; y &gt; 0</code></pre>";
        let text = extract_text_content(html);
        assert!(text.contains("if x < 10 && y > 0"));
    }
}
