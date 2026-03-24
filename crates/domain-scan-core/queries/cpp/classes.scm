;; C++ class declarations
(class_specifier
  name: (type_identifier) @class.name
  body: (field_declaration_list) @class.body
) @class.def

;; C++ struct declarations
(struct_specifier
  name: (type_identifier) @class.name
  body: (field_declaration_list) @class.body
) @class.def
