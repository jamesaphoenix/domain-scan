;; Python typing.Protocol classes (Python's interfaces)
(class_definition
  name: (identifier) @protocol.name
  superclasses: (argument_list) @protocol.bases
  body: (block) @protocol.body
) @protocol.def
