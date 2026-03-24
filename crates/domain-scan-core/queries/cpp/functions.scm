;; C++ top-level function definitions (not inside a class)
(function_definition
  type: (_) @function.return_type
  declarator: (function_declarator
    declarator: (identifier) @function.name
    parameters: (parameter_list) @function.params)
  body: (compound_statement) @function.body
) @function.def
