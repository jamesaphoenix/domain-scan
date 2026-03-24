;; C# record declarations (data transfer objects)
(record_declaration
  name: (identifier) @schema.name
) @schema.def

;; C# class declarations (filtered for [Table] / Entity Framework in Rust code)
(class_declaration
  name: (identifier) @schema.name
  body: (declaration_list) @schema.body
) @schema.def
