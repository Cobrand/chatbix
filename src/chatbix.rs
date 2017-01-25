use std::sync::RwLock;
use super::message::{NewMessage,Message};
use super::user::{ConnectedUser,CachedUsers};
use std::collections::VecDeque;
use chrono::NaiveDateTime;

use error::*;
use r2d2::Pool;

use r2d2_postgres::PostgresConnectionManager as PgConnection;

pub trait ChatbixInterface {
    type InitParams;
    fn new(init_params: Self::InitParams) -> Self;

    //fn new_message(&self, new_message: NewMessage);
    fn get_messages(&self, timestamp: Option<NaiveDateTime>, timestamp_end: Option<NaiveDateTime>, channels: &[&str]) -> Vec<Message>;

    fn refresh_connected_users(&self) {
        
    }
}

pub struct Chatbix<Connection> {
    connection: Connection,
    connected_users: RwLock<VecDeque<ConnectedUser>>,
    cached_users: RwLock<CachedUsers>
}

impl ChatbixInterface for Chatbix<Pool<PgConnection>> {

    // TODO: change InitParams into (&'a str,TlsMode<'h>)
    // so that the connection is init here instead of outside
    type InitParams = Pool<PgConnection>;

    fn new(init_params: Self::InitParams) -> Chatbix<Pool<PgConnection>> {
        Chatbix {
            connection: init_params,
            connected_users: RwLock::new(VecDeque::with_capacity(8)),
            cached_users: RwLock::new(CachedUsers::new()),
        }
    }

    fn get_messages(&self, timestamp: Option<NaiveDateTime>, timestamp_end: Option<NaiveDateTime>, channels: &[&str]) -> Vec<Message> {
        let pg = self.connection.get().unwrap(); // FIXME < if this is busy this will panic, 
            // you should send error 509 instead!
        let rows = match (timestamp, timestamp_end) {
            (None,None) =>
                pg.query("SELECT * FROM (SELECT * FROM chat_messages ORDER BY timestamp DESC LIMIT 150) as pote ORDER BY timestamp ASC;",
                                      &[]).unwrap(),
            (Some(timestamp),None) =>
                pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp >= $1 ORDER BY timestamp ASC;",
                                      &[&timestamp]).unwrap(),
            (Some(timestamp),Some(timestamp_end)) =>
                pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp >= $1 AND chat_messages.timestamp < $2 ORDER BY timestamp ASC;",
                                      &[&timestamp,&timestamp_end]).unwrap(),
            (None,Some(timestamp_end)) =>
                pg.query("SELECT * FROM (SELECT * FROM chat_messages WHERE chat_messages.timestamp < $1 ORDER BY timestamp DESC) as pote ORDER BY timestamp DESC;",
                                      &[&timestamp_end]).unwrap(),
        };
        // TODO : collect rows into Result<Vec, Err> instead,
        // so that it doesnt crash when the columns are changed (use get_opt instead of `get`)
        rows.into_iter().map(|row|{
            Message {
                id: row.get("id"),
                author: row.get("author"),
                timestamp: row.get("timestamp"),
                content: row.get("content"),
                tags: row.get("tags"),
                color: row.get("color"),
                channel: row.get("channel"),
            }
        }).collect()
    }
}
