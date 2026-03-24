;; C# ASP.NET controller / service detection via attributes
;; Matches classes with [ApiController], [Controller], [Route] attributes
(class_declaration
  name: (identifier) @service.name
  body: (declaration_list) @service.body
) @service.def
