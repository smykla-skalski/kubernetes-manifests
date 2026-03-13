(boolean_scalar) @boolean

(null_scalar) @constant.builtin

[
  (double_quote_scalar)
  (single_quote_scalar)
  (block_scalar)
  (string_scalar)
] @string

(escape_sequence) @string.escape

[
  (integer_scalar)
  (float_scalar)
] @number

(comment) @comment

[
  (anchor_name)
  (alias_name)
  (tag)
] @type

key: (flow_node
  [
    (plain_scalar
      (string_scalar))
    (double_quote_scalar)
    (single_quote_scalar)
  ] @property)

[
  ","
  "-"
  ":"
  ">"
  "?"
  "|"
] @punctuation.delimiter

[
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  "*"
  "&"
  "---"
  "..."
] @punctuation.special

; Resource identity keys
(block_mapping_pair
  key: (flow_node
    (plain_scalar
      (string_scalar) @keyword))
  (#any-of? @keyword "apiVersion" "kind"))

; Structural sections
(block_mapping_pair
  key: (flow_node
    (plain_scalar
      (string_scalar) @type))
  (#any-of? @type "metadata" "spec" "status" "template" "data" "stringData"))

; Common Kubernetes keys
(block_mapping_pair
  key: (flow_node
    (plain_scalar
      (string_scalar) @attribute))
  (#any-of? @attribute
    "name" "namespace" "labels" "annotations" "containers" "initContainers" "volumes" "volumeMounts"
    "ports" "env" "envFrom" "resources" "selector" "matchLabels" "matchExpressions" "replicas"
    "strategy" "image" "imagePullPolicy" "command" "args" "livenessProbe" "readinessProbe"
    "startupProbe" "requests" "limits" "serviceAccountName" "rules" "subjects" "roleRef"))
