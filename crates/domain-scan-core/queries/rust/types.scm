;; Rust struct definitions
(struct_item
  name: (type_identifier) @type.name
) @type.def

;; Rust enum definitions
(enum_item
  name: (type_identifier) @type.name
) @type.def

;; Rust type aliases
(type_item
  name: (type_identifier) @type.name
  type: (_) @type.value
) @type.def
