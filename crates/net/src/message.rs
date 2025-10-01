use crate::error::SocketError;
use crate::request::RequestData;
use crate::types::ReqId;
use crate::utils::split_once_space;

pub enum ParsedMsg<'a> {
    Req { data: RequestData<'a> },
    Res { id: String, raw_response: &'a str },
    Other(&'a str), // Línea cualquiera (compat/log)
}

pub fn parse_line(line: &str) -> Result<ParsedMsg<'_>, SocketError> {
    let msg = line.trim();

    if let Some(rest) = msg.strip_prefix("REQ ") {
        let request_data = RequestData::try_from(msg)?;

        return Ok(ParsedMsg::Req { data: request_data });
    }

    if let Some(rest) = msg.strip_prefix("RES ") {
        let (id_str, payload) = split_once_space(rest)?;

        let id = id_str
            .parse::<ReqId>()
            .map_err(|_| SocketError::BadMessage(msg.to_string()))?;

        return Ok(ParsedMsg::Res {
            id,
            raw_response: msg,
        });
    }

    Ok(ParsedMsg::Other(msg))
}
