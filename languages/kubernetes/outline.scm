; Resource-level outline: "Deployment demo-deployment"
; Matches kind value as context and metadata.name value as name
(block_mapping
  (block_mapping_pair
    key: (flow_node) @_kind_key
    (#eq? @_kind_key "kind")
    value: (flow_node) @context)
  (block_mapping_pair
    key: (flow_node) @_meta_key
    (#eq? @_meta_key "metadata")
    value: (block_node
      (block_mapping
        (block_mapping_pair
          key: (flow_node) @_name_key
          (#eq? @_name_key "name")
          value: (flow_node) @name))))) @item

; Resource-level outline for flow-style (KYAML)
(flow_mapping
  (flow_pair
    key: (flow_node) @_kind_key
    (#eq? @_kind_key "kind")
    value: (flow_node) @context)
  (flow_pair
    key: (flow_node) @_meta_key
    (#eq? @_meta_key "metadata")
    value: (flow_node
      (flow_mapping
        (flow_pair
          key: (flow_node) @_name_key
          (#eq? @_name_key "name")
          value: (flow_node) @name))))) @item

; Structural sections under a resource: spec, template, data (block)
(block_mapping_pair
  key: (flow_node
    (plain_scalar
      (string_scalar) @name))
  (#any-of? @name
    "spec" "status" "template" "data" "stringData" "rules" "subjects" "roleRef" "webhooks")
  value: (block_node)) @item

; Structural sections under a resource (flow-style)
(flow_pair
  key: (flow_node
    (plain_scalar
      (string_scalar) @name))
  (#any-of? @name
    "spec" "status" "template" "data" "stringData" "rules" "subjects" "roleRef" "webhooks")
  value: (flow_node)) @item

; Container names within containers/initContainers arrays (block)
(block_mapping_pair
  key: (flow_node) @_containers_key
  (#any-of? @_containers_key "containers" "initContainers" "ephemeralContainers")
  value: (block_node
    (block_sequence
      (block_sequence_item
        (block_node
          (block_mapping
            (block_mapping_pair
              key: (flow_node) @_name_key
              (#eq? @_name_key "name")
              value: (flow_node) @name))) @item))))

; Container names within containers/initContainers arrays (flow-style)
(flow_pair
  key: (flow_node) @_containers_key
  (#any-of? @_containers_key "containers" "initContainers" "ephemeralContainers")
  value: (flow_node
    (flow_sequence
      (flow_node
        (flow_mapping
          (flow_pair
            key: (flow_node) @_name_key
            (#eq? @_name_key "name")
            value: (flow_node) @name))) @item)))
