;; Python decorated class definitions (FastAPI, Flask, Django)
(decorated_definition
  (decorator) @service.decorator
  definition: (class_definition
    name: (identifier) @service.name
  ) @service.class_def
) @service.def

;; Python decorated function definitions (FastAPI route handlers)
(decorated_definition
  (decorator) @service.decorator
  definition: (function_definition
    name: (identifier) @service.name
  ) @service.func_def
) @service.def
