; Redact values of keys with secret-like names
(block_mapping_pair
  key: (flow_node
    (plain_scalar
      (string_scalar) @_key))
  value: (flow_node) @redact
  (#match? @_key "(?i)(password|passwd|secret|token|key|credential|auth|private)"))

; Redact all values nested under data: or stringData: (Secret manifests)
(block_mapping_pair
  key: (flow_node) @_data_key
  (#any-of? @_data_key "data" "stringData")
  value: (block_node
    (block_mapping
      (block_mapping_pair
        value: [
          (flow_node) @redact
          (block_node
            (block_scalar) @redact)
        ]))))

; Redact .dockerconfigjson
(block_mapping_pair
  key: (flow_node
    (plain_scalar
      (string_scalar) @_docker_key))
  value: [
    (flow_node) @redact
    (block_node
      (block_scalar) @redact)
  ]
  (#eq? @_docker_key ".dockerconfigjson"))
