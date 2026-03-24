;; Java record declarations and @Entity annotated classes
;; Records are data transfer objects
(record_declaration
  name: (identifier) @schema.name
  parameters: (formal_parameters) @schema.fields
) @schema.def

;; Regular class declarations (filtered for @Entity etc in Rust code)
(class_declaration
  name: (identifier) @schema.name
  body: (class_body) @schema.body
) @schema.def
