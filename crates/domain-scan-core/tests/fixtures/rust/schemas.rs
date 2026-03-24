#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDto {
    pub id: u64,
    pub name: String,
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub age: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct InternalState {
    counter: u64,
}
