((comment) @injection.content
  (#set! injection.language "comment"))

; GitHub actions: JavaScript for workflow scripting (inline and block)
(block_mapping
  (block_mapping_pair
    key: (flow_node) @_uses
    (#eq? @_uses "uses")
    value: (flow_node) @_actions_ghs
    (#match? @_actions_ghs "^actions/github-script"))
  (block_mapping_pair
    key: (flow_node) @_with
    (#eq? @_with "with")
    value: (block_node
      (block_mapping
        (block_mapping_pair
          key: (flow_node) @_run
          (#eq? @_run "script")
          value: [
            (flow_node
              (plain_scalar
                (string_scalar) @injection.content))
            (block_node
              (block_scalar) @injection.content)
          ]
          (#set! injection.language "javascript"))))))

; Shell highlighting for command/args array items
(block_mapping_pair
  key: (flow_node) @_cmd_key
  (#any-of? @_cmd_key "command" "args")
  value: (block_node
    (block_sequence
      (block_sequence_item
        (flow_node
          (plain_scalar
            (string_scalar) @injection.content))
        (#set! injection.language "bash")))))

; Shell highlighting for block scalar commands
(block_mapping_pair
  key: (flow_node) @_script_key
  (#any-of? @_script_key "command" "script")
  value: (block_node
    (block_scalar) @injection.content)
  (#set! injection.language "bash"))
