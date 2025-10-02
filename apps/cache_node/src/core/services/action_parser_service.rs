use app_core::utils::split_message;

use crate::core::domain::models::Command;

pub struct ActionParserService;

impl ActionParserService {
    pub fn parse(action: &str, line: &str) -> Command {
        let mut parts = split_message(line).into_iter();

        match action {
            "PING" => Command::Ping,
            "PUT" => {
                let key = parts.next().unwrap_or_default().to_string();
                let value = parts.next().unwrap_or_default().to_string();
                let ttl = parts.next().and_then(|s| s.parse::<u64>().ok());

                Command::Put { key, value, ttl }
            }
            "GET" => {
                let key = parts.next().unwrap_or_default().to_string();
                Command::Get { key }
            }
            _ => Command::Unknown(action.to_string()),
        }
    }
}
