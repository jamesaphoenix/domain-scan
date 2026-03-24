;; C++ method definitions inside class body
;; Methods are extracted from class body in Rust code via field_declaration_list traversal.
;; This query captures function_definitions that appear inside field_declaration_list.
(field_declaration_list
  (function_definition
    declarator: (function_declarator
      declarator: (_) @method.name)
  ) @method.def
)
