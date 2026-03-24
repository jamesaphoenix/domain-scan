;; Swift class/struct declarations (with class_body)
;; NOTE: tree-sitter-swift uses class_declaration for class, struct, enum, and extension.
;; We filter by the keyword child in Rust extraction code.
(class_declaration
  name: (type_identifier) @class.name
  (class_body) @class.body
) @class.def

;; Swift enum declarations (with enum_class_body)
(class_declaration
  name: (type_identifier) @class.name
  (enum_class_body) @class.body
) @class.def
