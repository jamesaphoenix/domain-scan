;; Kotlin class declarations (including data classes)
;; Both regular classes and data classes use class_declaration
;; Note: Kotlin uses (identifier) not (type_identifier) for names
(class_declaration
  name: (identifier) @class.name
) @class.def
