struct MyService {
    db: Database,
}

impl MyService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn process(&self, input: &str) -> Result<Output, Error> {
        todo!()
    }

    fn internal_helper(&self) -> bool {
        true
    }
}

impl EventHandler for MyService {
    fn handle(&self, event: Event) -> Result<(), Error> {
        Ok(())
    }

    fn name(&self) -> &str {
        "my_service"
    }
}

impl Clone for MyService {
    fn clone(&self) -> Self {
        todo!()
    }
}
