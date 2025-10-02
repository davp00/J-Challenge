use crate::utils::split_message;
use crate::{error::SocketError, types::ReqId};
use std::str::FromStr;

#[derive(Debug)]
pub struct ResponseData {
    req_id: ReqId,
    code: u16,
    payload: String,
}

impl ResponseData {
    #[inline]
    pub fn new(req_id: ReqId, code: u16, payload: String) -> Self {
        Self {
            req_id,
            code,
            payload,
        }
    }

    fn parse(s: &str) -> Result<Self, SocketError> {
        let parts = split_message(s);

        if parts.len() < 4 {
            return Err(SocketError::BadMessage(s.to_string()));
        }

        let code: u16 = parts[2]
            .parse()
            .map_err(|_| SocketError::BadRequest(format!("code {} not valid", parts[2])))?;

        Ok(Self::new(
            parts[1].to_string(),
            code,
            parts.get(3).copied().unwrap_or_default().to_string(),
        ))
    }
}

impl FromStr for ResponseData {
    type Err = SocketError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl ToString for ResponseData {
    fn to_string(&self) -> String {
        format!("RES {} {} \"{}\"\n", self.req_id, self.code, self.payload)
    }
}
