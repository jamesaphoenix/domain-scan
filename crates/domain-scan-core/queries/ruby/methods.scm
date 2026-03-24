;; Ruby method definitions
(method
  name: (_) @method.name
) @method.def

;; Ruby singleton (self.) methods
(singleton_method
  name: (_) @method.name
) @method.def
