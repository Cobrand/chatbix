use std::sync::{Arc,RwLock};
use super::chatbix::*;
use super::message::*;
use iron::*;

extern crate serde_json;

pub fn get_messages<C: ChatbixInterface>(req: &mut Request, chatbix: Arc<C>) -> Result<Response,IronError> {
    let messages : Vec<Message> = chatbix.get_messages(None,None,&[]);
    let json = serde_json::to_string(&messages).unwrap();
    Ok(Response::with((status::Ok,json)))
}
