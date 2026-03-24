pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub async fn fetch_data(url: &str) -> Result<String, Error> {
    todo!()
}

fn private_helper(input: &str) -> bool {
    input.is_empty()
}

pub fn process_items(items: Vec<Item>, filter: Option<&str>) -> Vec<Item> {
    todo!()
}

pub(crate) fn crate_visible() -> u64 {
    42
}
