; Redact values of keys with secret-like names (block + flow)
[
  (block_mapping_pair
    key: (flow_node
      (plain_scalar
        (string_scalar) @_key))
    value: (flow_node) @redact)
  (flow_pair
    key: (flow_node
      (plain_scalar
        (string_scalar) @_key))
    value: (flow_node) @redact)
]
(#match? @_key
  "(?i)(password|passwd|secret|token|key|credential|auth|private|oauth|bearer|api.key|api_key|apikey|client.secret|ssh|htpasswd|kubeconfig)")

; Redact all values nested under data: or stringData: (Secret manifests, block)
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

; Redact all values nested under data: or stringData: (Secret manifests, flow-style)
(flow_pair
  key: (flow_node) @_data_key
  (#any-of? @_data_key "data" "stringData")
  value: (flow_node
    (flow_mapping
      (flow_pair
        value: (flow_node) @redact))))

; Redact known Secret and TLS key names (block)
(block_mapping_pair
  key: (flow_node
    (plain_scalar
      (string_scalar) @_tls_key))
  value: [
    (flow_node) @redact
    (block_node
      (block_scalar) @redact)
  ]
  (#any-of? @_tls_key
    ".dockerconfigjson" ".dockercfg" "tls.crt" "tls.key" "ca.crt" "ca.key" "ssh-privatekey"
    "ssh-publickey"))

; Redact known Secret and TLS key names (flow-style)
(flow_pair
  key: (flow_node
    (plain_scalar
      (string_scalar) @_tls_key))
  value: (flow_node) @redact
  (#any-of? @_tls_key
    ".dockerconfigjson" ".dockercfg" "tls.crt" "tls.key" "ca.crt" "ca.key" "ssh-privatekey"
    "ssh-publickey"))
