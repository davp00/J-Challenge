use uuid::Uuid;

pub fn generate_short_id(len: usize) -> String {
    Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .chars()
        .take(len)
        .collect()
}
