;; Ruby require/require_relative/include calls
;; Handled in Rust code via call node matching
(call
  method: (identifier) @import.method
  arguments: (argument_list) @import.args
) @import.def
