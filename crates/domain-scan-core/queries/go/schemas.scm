;; Go struct type declarations with field tags (JSON, DB tags)
(type_declaration
  (type_spec
    name: (type_identifier) @schema.name
    type: (struct_type) @schema.body
  )
) @schema.def
