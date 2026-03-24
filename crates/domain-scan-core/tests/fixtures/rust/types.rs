pub struct User {
    pub id: u64,
    pub name: String,
    pub email: Option<String>,
    active: bool,
}

pub struct Config {
    pub host: String,
    pub port: u16,
}

pub enum Status {
    Active,
    Inactive,
    Pending(String),
}

pub type UserId = u64;
pub type Result<T> = std::result::Result<T, AppError>;
type InternalMap = HashMap<String, Vec<u8>>;
