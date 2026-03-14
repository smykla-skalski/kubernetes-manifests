(comment)+ @comment.around

; Select entire YAML document (Kubernetes resource)
(document) @class.around

; Select mapping pair with nested block content
(block_mapping_pair
  value: (block_node)) @function.around

; Select flow mapping pair with nested flow content
(flow_pair
  value: (flow_node
    [
      (flow_mapping)
      (flow_sequence)
    ])) @function.around
