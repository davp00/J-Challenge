use app_core::utils::split_message;

use crate::{error::SocketError, types::ReqId};
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Debug)]
pub struct RequestData<'a> {
    pub id: ReqId,
    pub action: &'a str,
    pub payload: &'a str,
}

impl<'a> RequestData<'a> {
    #[inline]
    pub fn new(id: ReqId, action: &'a str, payload: &'a str) -> Self {
        Self {
            id,
            action,
            payload,
        }
    }

    pub fn parse(s: &'a str) -> Result<Self, SocketError> {
        let parts = split_message(s);

        if parts.len() < 3 {
            return Err(SocketError::BadMessage(s.to_string()));
        }

        let (id, action, payload) = (parts[1], parts[2], parts.get(3).copied());

        if action.is_empty() || id.is_empty() {
            return Err(SocketError::BadRequest(s.to_string()));
        }

        Ok(Self::new(
            id.to_string(),
            action,
            payload.unwrap_or_default(),
        ))
    }
}

impl<'a> TryFrom<&'a str> for RequestData<'a> {
    type Error = SocketError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        RequestData::parse(s)
    }
}

impl<'a> ToString for RequestData<'a> {
    fn to_string(&self) -> String {
        format!("REQ {} {} \"{}\"\n", self.id, self.action, self.payload)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RequestDataInput<'a> {
    pub action: &'a str,
    pub payload: &'a str,
}

impl<'a> RequestDataInput<'a> {
    #[inline]
    pub fn new(action: &'a str, payload: &'a str) -> Self {
        Self { action, payload }
    }

    pub fn from_id(self, id: ReqId) -> RequestData<'a> {
        RequestData::<'a>::new(id, self.action, self.payload)
    }
}

#[derive(Clone)]
pub struct RequestDataOwned {
    pub id: ReqId,
    pub action: Arc<str>,
    pub payload: Arc<str>,
}

impl<'a> From<RequestData<'a>> for RequestDataOwned {
    fn from(d: RequestData<'a>) -> Self {
        Self {
            id: d.id,
            action: Arc::<str>::from(d.action),
            payload: Arc::<str>::from(d.payload),
        }
    }
}
