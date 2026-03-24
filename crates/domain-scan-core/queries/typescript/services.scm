;; Exported decorated classes (decorator is child of export_statement)
(export_statement
  (decorator) @service.decorator
  (class_declaration
    name: (type_identifier) @service.name
  ) @service.def
)

;; Exported abstract decorated classes
(export_statement
  (decorator) @service.decorator
  (abstract_class_declaration
    name: (type_identifier) @service.name
  ) @service.def
)

;; Non-exported decorated classes (decorator is child of class_declaration)
(class_declaration
  (decorator) @service.decorator
  name: (type_identifier) @service.name
) @service.def

(abstract_class_declaration
  (decorator) @service.decorator
  name: (type_identifier) @service.name
) @service.def
