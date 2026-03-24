pub trait EventHandler {
    fn handle(&self, event: Event) -> Result<(), Error>;
    fn name(&self) -> &str;
}

pub trait Repository<T> {
    fn find_by_id(&self, id: u64) -> Option<T>;
    fn save(&mut self, item: T) -> Result<(), Error>;
    fn delete(&self, id: u64) -> Result<(), Error>;
}

pub trait Serializable: Clone + Send {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(data: &[u8]) -> Self;
}

trait PrivateTrait {
    fn internal_method(&self);
}
