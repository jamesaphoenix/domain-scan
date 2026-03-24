;; Swift extension declarations
;; NOTE: tree-sitter-swift parses extensions as class_declaration with "extension" keyword.
;; The target type is in a user_type child (not type_identifier field).
;; We use the same class_declaration node and filter in Rust.
(class_declaration) @extension.def
