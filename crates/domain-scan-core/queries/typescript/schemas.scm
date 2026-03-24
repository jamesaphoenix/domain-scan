;; Schema definitions via member expression calls (Schema.Struct, z.object)
(variable_declarator
  name: (identifier) @schema.name
  value: (call_expression
    function: (member_expression
      object: (identifier) @schema.object
      property: (property_identifier) @schema.property)
    arguments: (arguments) @schema.fields)
) @schema.def

;; Schema definitions via direct function calls with table name (pgTable, etc.)
(variable_declarator
  name: (identifier) @schema.name
  value: (call_expression
    function: (identifier) @schema.function
    arguments: (arguments
      (string) @schema.table_name
      (_) @schema.fields))
) @schema.def
