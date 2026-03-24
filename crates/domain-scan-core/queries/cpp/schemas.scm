;; C++ struct declarations used as data transfer objects / schemas
;; Schema detection (POD structs) handled in Rust code
(struct_specifier
  name: (type_identifier) @schema.name
  body: (field_declaration_list) @schema.body
) @schema.def
