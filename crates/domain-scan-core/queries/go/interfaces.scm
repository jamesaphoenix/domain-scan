;; Go interface type declarations
;; Uses method_elem (not method_spec) per spec
(type_declaration
  (type_spec
    name: (type_identifier) @interface.name
    type: (interface_type) @interface.body
  )
) @interface.def
