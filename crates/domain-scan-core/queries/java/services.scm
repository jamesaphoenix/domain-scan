;; Java Spring Boot service/controller detection via annotations
;; Matches classes annotated with @Service, @Controller, @RestController, @Repository, @Component
(class_declaration
  name: (identifier) @service.name
  body: (class_body) @service.body
) @service.def
