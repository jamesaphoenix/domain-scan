use tonic::{Request, Response, Status};

pub struct MyServer {
    db: Database,
}

#[tonic::async_trait]
impl Greeter for MyServer {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        todo!()
    }
}

impl Default for MyServer {
    fn default() -> Self {
        Self { db: Database::new() }
    }
}
