extern crate bodyparser;
extern crate serde_json;

use std::sync::{Arc,RwLock};
use super::chatbix::*;
use super::message::*;
use iron::status;
use iron::prelude::*;
use urlencoded::{UrlEncodedQuery,UrlDecodingError};
use chrono::{NaiveDateTime, ParseError};

use error::*;

/// will both try to parse dates like 2017-01-19T22:56:16
/// and integers like 1485357232
fn timestamp_parse(t: &str) -> Result<NaiveDateTime> {
    let timestamp : StdResult<NaiveDateTime,_> = t.parse::<NaiveDateTime>();
    Ok(timestamp.or_else(|err|{
        let secs = t.parse::<i64>();
        match secs {
            Ok(secs) => Ok(NaiveDateTime::from_timestamp(secs,0)),
            Err(_) => Err(err),
        }
    })?)
}

#[derive(Debug, Serialize)]
struct JsonSuccess {
    status: &'static str,
    #[serde(skip_serializing_if="Option::is_none")]
    messages: Option<Vec<Message>>,
    #[serde(skip_serializing_if="Option::is_none")]
    auth_key: Option<String>,
}

impl JsonSuccess {
    pub fn empty() -> JsonSuccess {
        JsonSuccess {
            status: "success",
            messages: None,
            auth_key: None,
        }
    }

    pub fn with_messages(v: Vec<Message>) -> JsonSuccess {
        JsonSuccess {
            status: "success",
            messages: Some(v),
            auth_key: None,
        }
    }

    pub fn with_auth_key(auth_key: String) -> JsonSuccess {
        JsonSuccess {
            status: "success",
            messages: None,
            auth_key: Some(auth_key),
        }
    }

    pub fn to_string(&self) -> String {
        ::serde_json::to_string(&self).unwrap()
    }
}

pub fn new_message<I>(req: &mut Request, chatbix: Arc<Chatbix<I>>)-> IronResult<Response> where Chatbix<I>: ChatbixInterface {
    let message : Result<_> = req.get_ref::<bodyparser::Struct<NewMessage>>()
        .map_err(|e| Error::from_kind(ErrorKind::BodyparserError(e)));
    let message = chatbix_try!(message);
    let status : IronResult<()> = match message.as_ref() {
        None => return Error::from_kind(ErrorKind::NoJsonBodyDetected).into(),
        Some(new_message) => Ok(chatbix_try!(chatbix.new_message(new_message))),
    };
    status.map(|_| Response::with(JsonSuccess::empty().to_string()))
}

pub fn heartbeat<I>(req: &mut Request, chatbix: Arc<Chatbix<I>>)-> IronResult<Response> where Chatbix<I>: ChatbixInterface {
    unimplemented!()
}

pub fn get_messages<I>(req: &mut Request, chatbix: Arc<Chatbix<I>>)-> IronResult<Response> where Chatbix<I>: ChatbixInterface {
    let mut channels : Vec<String> = Vec::new();
    let mut include_default_channel = true;
    let (timestamp, timestamp_end) = match req.get_ref::<UrlEncodedQuery>() {
        Ok(hashmap) => {
            if let Some(tmp_chans) = hashmap.get("channels").and_then(|c| c.get(0)) {
                channels = tmp_chans.split(',').map(|s:&str| s.to_owned()).collect::<Vec<String>>();
            };
            if let Some(tmp_chans) = hashmap.get("channel") {
                for c in tmp_chans {
                    channels.push(c.clone());
                }
            };
            if hashmap.get("no_default_channel").is_some() {
                include_default_channel = false;
            };
            match (hashmap.get("timestamp"),hashmap.get("timestamp_end")) {
                (None,None) => {
                    (None,None)
                },
                (Some(timestamps),None) => {
                    let timestamp = chatbix_try!(timestamp_parse(timestamps.get(0).unwrap())); 
                    (Some(timestamp),None)
                },
                (None,Some(timestamps_end)) => {
                    let timestamp_end = chatbix_try!(timestamp_parse(timestamps_end.get(0).unwrap()));
                    (None,Some(timestamp_end))
                },
                (Some(timestamps),Some(timestamps_end)) => {
                    let timestamp = chatbix_try!(timestamp_parse(timestamps.get(0).unwrap()));
                    let timestamp_end = chatbix_try!(timestamp_parse(timestamps_end.get(0).unwrap()));
                    (Some(timestamp),Some(timestamp_end))
                }
            }
        },
        Err(UrlDecodingError::EmptyQuery) => (None,None),
        Err(UrlDecodingError::BodyError(body_error)) => {
            return Err(IronError::new(body_error,(status::BadRequest)))
        },
    };
    let messages = chatbix_try!(chatbix.get_messages(timestamp,timestamp_end,channels,include_default_channel));
    Ok(Response::with((status::Ok,JsonSuccess::with_messages(messages).to_string())))
}

#[derive(Debug, Deserialize)]
struct LoginPayload {
    username: String,
    password: String,
}

pub fn register<I>(req: &mut Request, chatbix: Arc<Chatbix<I>>)-> IronResult<Response> where Chatbix<I>: ChatbixInterface {
    let login_payload : Result<_> = req.get_ref::<bodyparser::Struct<LoginPayload>>()
        .map_err(|e| Error::from_kind(ErrorKind::BodyparserError(e)));
    let login_payload = chatbix_try!(login_payload);
    let auth_key = match login_payload.as_ref() {
        None => return Error::from_kind(ErrorKind::NoJsonBodyDetected).into(),
        Some(p) => chatbix_try!(chatbix.register(p.username.as_str(), p.password.as_str())),
    };
    Ok(Response::with((status::Ok,JsonSuccess::with_auth_key(auth_key).to_string())))
}

pub fn login<I>(req: &mut Request, chatbix: Arc<Chatbix<I>>)-> IronResult<Response> where Chatbix<I>: ChatbixInterface {
    let login_payload : Result<_> = req.get_ref::<bodyparser::Struct<LoginPayload>>()
        .map_err(|e| Error::from_kind(ErrorKind::BodyparserError(e)));
    let login_payload = chatbix_try!(login_payload);
    let auth_key = match login_payload.as_ref() {
        None => return Error::from_kind(ErrorKind::NoJsonBodyDetected).into(),
        Some(p) => chatbix_try!(chatbix.login(p.username.as_str(), p.password.as_str())),
    };
    Ok(Response::with((status::Ok,JsonSuccess::with_auth_key(auth_key).to_string())))
}


#[derive(Debug, Deserialize)]
struct LogoutPayload {
    username: String,
    auth_key: String,
}

pub fn logout<I>(req: &mut Request, chatbix: Arc<Chatbix<I>>)-> IronResult<Response> where Chatbix<I>: ChatbixInterface {
    let logout_payload : Result<_> = req.get_ref::<bodyparser::Struct<LogoutPayload>>()
        .map_err(|e| Error::from_kind(ErrorKind::BodyparserError(e)));
    let logout_payload = chatbix_try!(logout_payload);
    match logout_payload.as_ref() {
        None => return Error::from_kind(ErrorKind::NoJsonBodyDetected).into(),
        Some(p) => chatbix_try!(chatbix.logout(p.username.as_str(), p.auth_key.as_str())),
    };
    Ok(Response::with((status::Ok,JsonSuccess::empty().to_string())))
}
