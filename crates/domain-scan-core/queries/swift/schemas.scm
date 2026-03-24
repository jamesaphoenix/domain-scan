;; Swift struct/class declarations for Codable detection (with class_body)
;; Schema detection (Codable, CodingKeys) handled in Rust code
(class_declaration
  name: (type_identifier) @schema.name
  (class_body) @schema.body
) @schema.def

;; Swift enum declarations for Codable detection (with enum_class_body)
(class_declaration
  name: (type_identifier) @schema.name
  (enum_class_body) @schema.body
) @schema.def
