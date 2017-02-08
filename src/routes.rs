extern crate bodyparser;
extern crate serde_json;

use std::sync::Arc;
use super::chatbix::*;
use super::message::*;
use super::user::ConnectedUser;
use super::utils::timestamp_parse;
use iron::status;
use iron::prelude::*;
use urlencoded::{UrlEncodedQuery,UrlDecodingError};

use error::*;

#[derive(Debug, Serialize)]
struct JsonSuccess {
    status: &'static str,
    #[serde(skip_serializing_if="Option::is_none")]
    users_connected: Option<Vec<ConnectedUser>>,
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
            users_connected: None,
            auth_key: None,
        }
    }

    pub fn with_messages(v: Vec<Message>) -> JsonSuccess {
        JsonSuccess {
            status: "success",
            messages: Some(v),
            users_connected: None,
            auth_key: None,
        }
    }

    pub fn with_messages_and_connected(v: Vec<Message>, users: Vec<ConnectedUser>) -> JsonSuccess {
        JsonSuccess {
            status: "success",
            messages: Some(v),
            users_connected: Some(users),
            auth_key: None,
        }
    }

    pub fn with_auth_key(auth_key: String) -> JsonSuccess {
        JsonSuccess {
            status: "success",
            messages: None,
            users_connected: None,
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
    status.map(|_| Response::with((status::Ok,JsonSuccess::empty().to_string())))
}

pub fn heartbeat<I>(req: &mut Request, chatbix: Arc<Chatbix<I>>)-> IronResult<Response> where Chatbix<I>: ChatbixInterface {
    let mut channels : Vec<String> = Vec::new();
    let mut include_default_channel : bool = true;
    let mut credentials : Option<(String,Option<String>,bool)> = None;
    let interval = match req.get_ref::<UrlEncodedQuery>() {
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
            let username = hashmap.get("username").map(|u| u.get(0).unwrap().clone());
            let auth_key = hashmap.get("auth_key").map(|k| k.get(0).unwrap().clone());
            let active = hashmap.get("active").map(|active| {
                let active = active.get(0).unwrap();
                !(active == "false" || active == "FALSE" || active == "0")
            }).unwrap_or(true);
            if let Some(username) = username {
                credentials = Some((username,auth_key.clone(),active));
            };
            match (hashmap.get("timestamp"),hashmap.get("message_id")) {
                (None,None) => Interval::default(),
                (Some(timestamps),None) => Interval::AllFromTimestamp(chatbix_try!(timestamp_parse(timestamps.get(0).unwrap()))),
                (_,Some(message_id)) => Interval::AllFromId(chatbix_try!(message_id.get(0).unwrap().parse::<i32>().map_err(|e| Error::from(e)))),
            }
        },
        Err(UrlDecodingError::EmptyQuery) => Interval::default(),
        Err(UrlDecodingError::BodyError(body_error)) => {
            return Err(IronError::new(body_error,(status::BadRequest)))
        },
    };
    let connected_users = match credentials {
        Some((username,Some(auth_key),active)) => {
            chatbix_try!(chatbix.heartbeat_mut(&*username, Some(&*auth_key), active))
        },
        Some((username,None,active)) => {
            chatbix_try!(chatbix.heartbeat_mut(&*username, None, active))
        },
        None => chatbix.heartbeat()
    };
    let messages = chatbix_try!(chatbix.get_messages(interval,channels,include_default_channel));
    Ok(Response::with((status::Ok,JsonSuccess::with_messages_and_connected(messages, connected_users).to_string())))
}
// ^ TODO: refactor this with heartbeat

pub fn get_messages<I>(req: &mut Request, chatbix: Arc<Chatbix<I>>)-> IronResult<Response> where Chatbix<I>: ChatbixInterface {
    let mut channels : Vec<String> = Vec::new();
    let mut include_default_channel = true;
    let interval = match req.get_ref::<UrlEncodedQuery>() {
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
            match (hashmap.get("message_id"),hashmap.get("timestamp"),hashmap.get("timestamp_end")) {
                (None,Some(timestamps),None) => {
                    let timestamp = chatbix_try!(timestamp_parse(timestamps.get(0).unwrap())); 
                    Interval::AllFromTimestamp(timestamp)
                },
                (None,Some(timestamps),Some(timestamps_end)) => {
                    let timestamp = chatbix_try!(timestamp_parse(timestamps.get(0).unwrap()));
                    let timestamp_end = chatbix_try!(timestamp_parse(timestamps_end.get(0).unwrap()));
                    Interval::FromToTimestamp(timestamp, timestamp_end)
                },
                (Some(message_id),_,_) => {
                    let message_id = chatbix_try!(message_id.get(0).unwrap().parse::<i32>()
                        .map_err(|e| Error::from(e)));
                    Interval::AllFromId(message_id)
                },
                (_,_,_) => {
                    Interval::default()
                },
            }
        },
        Err(UrlDecodingError::EmptyQuery) => Interval::default(),
        Err(UrlDecodingError::BodyError(body_error)) => {
            return Err(IronError::new(body_error,(status::BadRequest)))
        },
    };
    let messages = chatbix_try!(chatbix.get_messages(interval,channels,include_default_channel));
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
struct DelMessage {
    pub message_id: i32,
    pub username: String,
    pub auth_key: String,
}

/// admin only : taht is why auth_key is required
pub fn delete_message<I>(req: &mut Request, chatbix: Arc<Chatbix<I>>)-> IronResult<Response> where Chatbix<I>: ChatbixInterface {
    let del_message : Result<_> = req.get_ref::<bodyparser::Struct<DelMessage>>()
        .map_err(|e| Error::from_kind(ErrorKind::BodyparserError(e)));
    let del_message = chatbix_try!(del_message);
    match del_message.as_ref() { 
        None => return Error::from_kind(ErrorKind::NoJsonBodyDetected).into(),
        Some(del_message) => chatbix_try!(chatbix.try_del(del_message.username.as_str(), del_message.auth_key.as_str(), del_message.message_id))
    };
    Ok(Response::with((status::Ok,JsonSuccess::empty().to_string())))
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
