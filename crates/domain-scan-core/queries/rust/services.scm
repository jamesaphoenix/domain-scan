;; Rust service detection via attribute macros
;; actix-web: #[get("/path")], #[post("/path")], etc.
;; axum: Router::new().route("/path", get(handler))
;; tonic/gRPC: #[tonic::async_trait] impl MyService for MyServer

;; Impl blocks with attributes (e.g., tonic gRPC service impls)
(impl_item
  type: (_) @service.target
  body: (declaration_list) @service.body
) @service.def
