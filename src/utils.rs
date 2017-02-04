use chrono::NaiveDateTime;
use std::result::Result as StdResult;
use error::*;
use serde::Serializer;
use chrono::{UTC,Timelike};

pub fn timestamp_ser<S>(time: &NaiveDateTime, serializer: &mut S) -> StdResult<(), S::Error> where S: Serializer {
    serializer.serialize_i64(time.timestamp())
}

/// will both try to parse dates like 2017-01-19T22:56:16
/// and integers like 1485357232
pub fn timestamp_parse(t: &str) -> Result<NaiveDateTime> {
    let timestamp : StdResult<NaiveDateTime,_> = t.parse::<NaiveDateTime>();
    Ok(timestamp.or_else(|err|{
        let secs = t.parse::<i64>();
        match secs {
            Ok(secs) => Ok(NaiveDateTime::from_timestamp(secs,0)),
            Err(_) => Err(err),
        }
    })?)
}

pub fn now() -> NaiveDateTime {
    UTC::now().naive_utc().with_nanosecond(0).unwrap()
}
