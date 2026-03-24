;; Go struct type declarations
(type_declaration
  (type_spec
    name: (type_identifier) @struct.name
    type: (struct_type) @struct.body
  )
) @struct.def
