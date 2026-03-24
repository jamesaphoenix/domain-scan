;; Swift service detection via class declarations (with class_body)
;; Services are identified by attributes or naming conventions in Rust code
(class_declaration
  name: (type_identifier) @service.name
  (class_body) @service.body
) @service.def

;; Swift service detection via enum declarations (with enum_class_body)
(class_declaration
  name: (type_identifier) @service.name
  (enum_class_body) @service.body
) @service.def
