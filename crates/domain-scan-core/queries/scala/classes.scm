;; Scala class definitions (including case classes)
(class_definition
  name: (identifier) @class.name
  body: (template_body)? @class.body
) @class.def

;; Scala object definitions (singletons)
(object_definition
  name: (identifier) @class.name
  body: (template_body)? @class.body
) @class.def
