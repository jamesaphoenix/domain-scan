;; C++ class specifiers (used to detect pure virtual = abstract base classes)
;; The Rust extraction code checks for pure_virtual_clause children
(class_specifier
  name: (type_identifier) @interface.name
  body: (field_declaration_list) @interface.body
) @interface.def
