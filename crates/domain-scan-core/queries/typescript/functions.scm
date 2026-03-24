;; Function declarations
(function_declaration
  name: (identifier) @function.name
) @function.def

;; Arrow functions assigned to variables
(variable_declarator
  name: (identifier) @function.name
  value: (arrow_function) @function.value
) @function.def

;; Function expressions assigned to variables
(variable_declarator
  name: (identifier) @function.name
  value: (function_expression) @function.value
) @function.def
