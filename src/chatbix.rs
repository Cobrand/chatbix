use std::sync::RwLock;
use super::message::{NewMessage,Message};
use super::user::{ConnectedUser,CachedUsers,UserConnectionStatus};
use std::collections::VecDeque;
use chrono::{NaiveDateTime,UTC};

use error::*;
use r2d2::{Pool,PooledConnection};

use r2d2_postgres::PostgresConnectionManager as PgConnection;

fn now() -> NaiveDateTime {
    UTC::now().naive_utc()
}

pub trait ChatbixInterface {
    type InitParams;
    fn new(init_params: Self::InitParams) -> Self;

    //fn new_message(&self, new_message: NewMessage);
    fn get_messages<V: AsRef<[String]>>(&self, timestamp: Option<NaiveDateTime>, timestamp_end: Option<NaiveDateTime>, channels: V, include_default_channel: bool) -> Result<Vec<Message>>;

    fn new_message(&self, new_message: &NewMessage) -> Result<()>;
}

pub struct Chatbix<Connection> {
    connection: Connection,
    connected_users: RwLock<VecDeque<ConnectedUser>>,
    cached_users: RwLock<CachedUsers>
}

impl<C> Chatbix<C> {
    fn on_new_message(&self) {

    }

    fn refresh_users(&self) {

    }

    fn check_user_auth_key(&self, username: &str, auth_key: &str) -> UserConnectionStatus {
        let cached_users = self.cached_users.read().unwrap();
        cached_users.check(username, auth_key)
    }
}

impl ChatbixInterface for Chatbix<Pool<PgConnection>> {

    // TODO: change InitParams into (&'a str,TlsMode<'h>)
    // so that the connection is init here instead of outside
    // UPDATE: ^not sure that it's the right thing to do ...
    type InitParams = Pool<PgConnection>;

    fn new(init_params: Self::InitParams) -> Chatbix<Pool<PgConnection>> {
        Chatbix {
            connection: init_params,
            connected_users: RwLock::new(VecDeque::with_capacity(8)),
            cached_users: RwLock::new(CachedUsers::new()),
        }
    }
    
    fn new_message(&self, new_message: &NewMessage) -> Result<()> {
        let pg : PooledConnection<_> = try!(self.connection.get().map_err(|_| Error::from_kind(ErrorKind::DatabaseBusy)));
        let timestamp : NaiveDateTime = now();
        let tags : i32= new_message.tags.unwrap_or(0) & 0b000_0000_0000_0000_0000_0000_1111_1110i32; // see User.tags for more info
        pg.query("INSERT INTO chat_messages (author, timestamp, content, tags, color, channel) \
                  VALUES ($1, $2, $3, $4, $5, $6)",
                  &[&new_message.author, &timestamp, &new_message.content, &tags, &new_message.color, &new_message.channel]).unwrap();
        Ok(())
    }

    fn get_messages<V: AsRef<[String]>>(&self, timestamp: Option<NaiveDateTime>, timestamp_end: Option<NaiveDateTime>, channels: V, include_default_channel: bool) -> Result<Vec<Message>> {
        let pg : PooledConnection<_> = try!(self.connection.get().map_err(|_| Error::from_kind(ErrorKind::DatabaseBusy))); 
        let rows = match (timestamp, timestamp_end) {
            (None,None) =>
                if include_default_channel {
                    pg.query("SELECT * FROM (SELECT * FROM chat_messages WHERE channel IS NULL OR channel = ANY ($1) ORDER BY timestamp DESC LIMIT 150) as pote ORDER BY timestamp ASC;",
                             &[&channels.as_ref()]).unwrap()
                } else {
                    pg.query("SELECT * FROM (SELECT * FROM chat_messages WHERE channel = ANY ($1) ORDER BY timestamp DESC LIMIT 150) as pote ORDER BY timestamp ASC;",
                             &[&channels.as_ref()]).unwrap()
                },
            (Some(timestamp),None) =>
                if include_default_channel {
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp >= $1 AND (channel IS NULL OR channel = ANY ($2)) ORDER BY timestamp ASC;",
                             &[&timestamp,&channels.as_ref()]).unwrap()
                } else {
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp >= $1 AND channel = ANY ($2) ORDER BY timestamp ASC;",
                             &[&timestamp,&channels.as_ref()]).unwrap()
                },
            (Some(timestamp),Some(timestamp_end)) =>
                if include_default_channel {
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp >= $1 AND chat_messages.timestamp < $2 AND (channel IS NULL OR channel = ANY ($3)) ORDER BY timestamp ASC;",
                             &[&timestamp,&timestamp_end,&channels.as_ref()]).unwrap()
                } else { 
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp >= $1 AND chat_messages.timestamp < $2 AND channel = ANY ($3) ORDER BY timestamp ASC;",
                             &[&timestamp,&timestamp_end,&channels.as_ref()]).unwrap()
                },
            (None,Some(timestamp_end)) =>
                if include_default_channel {
                    pg.query("SELECT * FROM (SELECT * FROM chat_messages WHERE chat_messages.timestamp < $1 AND (channel IS NULL OR channel = ANY ($2)) ORDER BY timestamp DESC) as pote ORDER BY timestamp ASC;",
                             &[&timestamp_end,&channels.as_ref()]).unwrap()
                } else {
                    pg.query("SELECT * FROM (SELECT * FROM chat_messages WHERE chat_messages.timestamp < $1 AND (channel = ANY ($2)) ORDER BY timestamp DESC) as pote ORDER BY timestamp ASC;",
                             &[&timestamp_end,&channels.as_ref()]).unwrap()
                },
        };
        // TODO : collect rows into Result<Vec, Err> instead,
        // so that it doesnt crash when the columns are changed (use get_opt instead of `get`)
        Ok(rows.into_iter().map(|row|{
            Message {
                id: row.get("id"),
                author: row.get("author"),
                timestamp: row.get("timestamp"),
                content: row.get("content"),
                tags: row.get("tags"),
                color: row.get("color"),
                channel: row.get("channel"),
            }
        }).collect())
    }
}
