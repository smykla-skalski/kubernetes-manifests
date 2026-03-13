(comment)+ @comment.around

; Select entire YAML document (Kubernetes resource)
(document) @class.around

; Select mapping pair with nested block content
(block_mapping_pair
  value: (block_node)) @function.around
