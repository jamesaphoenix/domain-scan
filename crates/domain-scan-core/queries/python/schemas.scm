;; Python class definitions that may be Pydantic, dataclass, TypedDict, SQLAlchemy models
(class_definition
  name: (identifier) @schema.name
  body: (block) @schema.body
) @schema.def
