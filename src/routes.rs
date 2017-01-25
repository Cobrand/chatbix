use std::sync::{Arc,RwLock};
use super::chatbix::*;
use super::message::*;
use iron::*;
use urlencoded::{UrlEncodedQuery,UrlDecodingError};
use chrono::{NaiveDateTime, ParseError};

use error::*;

extern crate bodyparser;
extern crate serde_json;

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

pub fn get_messages<C: ChatbixInterface>(req: &mut Request, chatbix: Arc<C>) -> IronResult<Response> {
    let mut channels : Vec<String> = Vec::new();
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
            match (hashmap.get("timestamp"),hashmap.get("timestamp_end")) {
                (None,None) => {
                    (None,None)
                },
                (Some(timestamps),None) => {
                    let timestamp = try!(iron_result(timestamp_parse(timestamps.get(0).unwrap()))); 
                    (Some(timestamp),None)
                },
                (None,Some(timestamps_end)) => {
                    let timestamp_end = try!(iron_result(timestamp_parse(timestamps_end.get(0).unwrap())));
                    (None,Some(timestamp_end))
                },
                (Some(timestamps),Some(timestamps_end)) => {
                    let timestamp = try!(iron_result(timestamp_parse(timestamps.get(0).unwrap())));
                    let timestamp_end = try!(iron_result(timestamp_parse(timestamps_end.get(0).unwrap())));
                    (Some(timestamp),Some(timestamp_end))
                }
            }
        },
        Err(UrlDecodingError::EmptyQuery) => (None,None),
        Err(UrlDecodingError::BodyError(body_error)) => {
            return Err(IronError::new(body_error,(status::BadRequest)))
        },
    };
    let messages = chatbix.get_messages(timestamp,timestamp_end,channels);
    let json = serde_json::to_string(&messages).unwrap();
    Ok(Response::with((status::Ok,json)))
}
