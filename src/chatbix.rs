use std::sync::RwLock;
use super::message::{NewMessage,Message};
use super::user::{ConnectedUser,ConnectedUsers,CachedUsers,UserConnectionStatus};
use chrono::NaiveDateTime;
use crypto::digest::Digest;
use crypto::sha2::Sha512;
use super::utils::now;

use error::*;
use r2d2::{Pool,PooledConnection};

use r2d2_postgres::PostgresConnectionManager as PgConnection;

pub enum Interval {
    AllFromId(i32),
    AllFromTimestamp(NaiveDateTime),
    FromToTimestamp(NaiveDateTime, NaiveDateTime),
    Last(i64),
}

impl Default for Interval {
    fn default() -> Interval {
        Interval::Last(150)
    }
}

pub trait ChatbixInterface {
    type InitParams;
    fn new(init_params: Self::InitParams) -> Self;

    fn get_messages<V: AsRef<[String]>>(&self, interval: Interval, channels: V, include_default_channel: bool) -> Result<Vec<Message>>;

    fn new_message(&self, new_message: &NewMessage) -> Result<()>;

    /// forces message deletion
    /// You should probably use try_del instead if coming from a user
    fn delete_message(&self, id: i32) -> Result<()>;

    /// returns some auth_key
    fn register(&self, username: &str, password: &str) -> Result<String>;

    /// return auth_key
    fn login(&self, username: &str, password: &str) -> Result<String>;

    /// Do a fulltext search on all the messages
    fn fulltext_search(&self, query: &str, limit: u32) -> Result<Vec<Match>>;
}

#[derive(Debug, Serialize)]
pub struct Match {
    pub user: String,
    pub message: String,
    pub rank: f32,
}

pub struct Chatbix<Connection> {
    connection: Connection,
    connected_users: RwLock<ConnectedUsers>,
    cached_users: RwLock<CachedUsers>
}

impl<C> Chatbix<C> {
    pub fn logout(&self, username: &str, auth_key: &str) -> Result<()> {
        let mut cached_users = self.cached_users.write().unwrap();
        cached_users.logout(username, auth_key)
    }

    pub fn refresh_users(&self) {
        let mut connected_users = self.connected_users.write().unwrap();
        connected_users.refresh();
    }

    fn check_user_auth_key(&self, username: &str, auth_key: &str) -> UserConnectionStatus {
        let cached_users = self.cached_users.read().unwrap();
        cached_users.check(username, auth_key)
    }
}

impl<C> Chatbix<C> where Chatbix<C>:ChatbixInterface {
    pub fn heartbeat(&self) -> Vec<ConnectedUser> {
        self.connected_users.read().unwrap().as_vec()
    }

    pub fn heartbeat_mut(&self, username: &str, auth_key: Option<&str>, active: bool) -> Result<Vec<ConnectedUser>> {
        let mut connected_users = self.connected_users.write().unwrap();
        let logged_in = match auth_key {
            Some(auth_key) => {
                match self.check_user_auth_key(username,auth_key) {
                    UserConnectionStatus::Connected(_) => true,
                    _ => false,
                }
            },
            None => false
        };
        connected_users.update(username, logged_in, active);
        Ok(connected_users.as_vec())
    }

    /// checks if user is allowed to delete first
    pub fn try_del(&self, username: &str, auth_key: &str, message_id: i32) -> Result<()> {
        use super::user::UserConnectionStatus::*;
        match self.check_user_auth_key(username, auth_key) {
            AuthFailed => Err(Error::from_kind(ErrorKind::InvalidAuthKey)),
            NotLoggedIn => Err(Error::from_kind(ErrorKind::NotLoggedIn)),
            Connected(false) => Err(Error::from_kind(ErrorKind::Forbidden)),
            Connected(true) => {
                self.delete_message(message_id)
            }
        }
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
            connected_users: RwLock::new(ConnectedUsers::new()),
            cached_users: RwLock::new(CachedUsers::new()),
        }
    }

    fn new_message(&self, new_message: &NewMessage) -> Result<()> {
        let timestamp : NaiveDateTime = now();
        let pg : PooledConnection<_> = try!(self.connection.get().map_err(|_| Error::from_kind(ErrorKind::DatabaseBusy)));
        let mut tags : i32= new_message.tags.unwrap_or(0) & 0b000_0000_0000_0000_0000_0011_1111_1110i32; // see User.tags for more info
        if let Some(ref auth_key) = new_message.auth_key {
            let cached_users = self.cached_users.read().unwrap();
            match cached_users.check(&*new_message.username, &*auth_key) {
                UserConnectionStatus::NotLoggedIn => bail!(ErrorKind::NotLoggedIn),
                UserConnectionStatus::AuthFailed => bail!(ErrorKind::InvalidAuthKey),
                UserConnectionStatus::Connected(_) => {
                    tags |= 1;
                }
            }
        };
        pg.query("INSERT INTO chat_messages (author, timestamp, content, tags, color, channel) \
                  VALUES ($1, $2, $3, $4, $5, $6)",
                  &[&new_message.username, &timestamp, &new_message.content, &tags, &new_message.color, &new_message.channel]).unwrap();
        Ok(())
    }

    fn delete_message(&self, id: i32) -> Result<()> {
        let pg : PooledConnection<_> = try!(self.connection.get().map_err(|_| Error::from_kind(ErrorKind::DatabaseBusy)));
        pg.query("DELETE FROM chat_messages WHERE id = $1",&[&id]).unwrap();
        Ok(())
    }

    fn get_messages<V: AsRef<[String]>>(&self, interval: Interval, channels: V, include_default_channel: bool) -> Result<Vec<Message>> {
        let pg : PooledConnection<_> = try!(self.connection.get().map_err(|_| Error::from_kind(ErrorKind::DatabaseBusy))); 
        let rows = match interval {
            Interval::Last(last) => {
                if include_default_channel {
                    pg.query("SELECT * FROM (SELECT * FROM chat_messages WHERE channel IS NULL OR channel = ANY ($1) ORDER BY timestamp DESC LIMIT $2) as pote ORDER BY timestamp ASC;",
                             &[&channels.as_ref(),&last])
                } else {
                    pg.query("SELECT * FROM (SELECT * FROM chat_messages WHERE channel = ANY ($1) ORDER BY timestamp DESC LIMIT $2) as pote ORDER BY timestamp ASC;",
                             &[&channels.as_ref(),&last])
                }
            },
            Interval::AllFromTimestamp(timestamp) =>
                if include_default_channel {
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp > $1 AND (channel IS NULL OR channel = ANY ($2)) ORDER BY timestamp ASC;",
                             &[&timestamp,&channels.as_ref()])
                } else {
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp > $1 AND channel = ANY ($2) ORDER BY timestamp ASC;",
                             &[&timestamp,&channels.as_ref()])
                },
            Interval::FromToTimestamp(timestamp, timestamp_end) =>
                if include_default_channel {
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp > $1 AND chat_messages.timestamp < $2 AND (channel IS NULL OR channel = ANY ($3)) ORDER BY timestamp ASC;",
                             &[&timestamp,&timestamp_end,&channels.as_ref()])
                } else { 
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp > $1 AND chat_messages.timestamp < $2 AND channel = ANY ($3) ORDER BY timestamp ASC;",
                             &[&timestamp,&timestamp_end,&channels.as_ref()])
                },
            Interval::AllFromId(id) =>
                if include_default_channel {
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.id > $1 AND (channel IS NULL OR channel = ANY ($2)) ORDER BY timestamp ASC;",
                             &[&id,&channels.as_ref()])
                } else {
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.id > $1 AND channel = ANY ($2) ORDER BY timestamp ASC;",
                             &[&id,&channels.as_ref()])
                },
        }.expect("PG Query Failed");
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
    
    fn register(&self, username: &str, password: &str) -> Result<String> {
        let pg : PooledConnection<_> = try!(self.connection.get().map_err(|_| Error::from_kind(ErrorKind::DatabaseBusy)));
        let rows = pg.query("SELECT COUNT(*) as count FROM chat_users WHERE username = $1;",&[&username]).unwrap();
        let count : i64 = rows.into_iter().next().unwrap().get("count");
        if count == 0 {
            // username is available !
            let mut hasher = Sha512::new();
            hasher.input_str(password);
            let hex_password = hasher.result_str();
            let password = hex_password.split_at(64).0;
            pg.query("INSERT INTO chat_users (username, password) VALUES ($1, $2)",&[&username,&password]).unwrap();
            {
                let mut cached_users = self.cached_users.write().unwrap();
                Ok(cached_users.login(username, false))
            }
        } else {
            Err(Error::from_kind(ErrorKind::UsernameInUse))
        }
    }
    
    /// return auth_key
    fn login(&self, username: &str, password: &str) -> Result<String> {
        let pg : PooledConnection<_> = try!(self.connection.get().map_err(|_| Error::from_kind(ErrorKind::DatabaseBusy)));
        let mut hasher = Sha512::new();
        hasher.input_str(&*password);
        let hex_password = hasher.result_str();
        let password = hex_password.split_at(64).0;
        let rows = pg.query("SELECT admin FROM chat_users WHERE username = $1 AND password = $2",&[&username,&password]).unwrap();
        let admin : Option<bool> = rows.into_iter().next().map(|r| r.get("admin"));
        match admin {
            Some(a) => {
                let mut cached_users = self.cached_users.write().unwrap();
                Ok(cached_users.login(username, a))
            },
            None => bail!(ErrorKind::InvalidCredentials),
        }
    }

    fn fulltext_search(&self, query: &str, limit: u32) -> Result<Vec<Match>> {
        let pg : PooledConnection<_> = try!(self.connection.get().map_err(|_| Error::from_kind(ErrorKind::DatabaseBusy)));
        let rows = pg.query("select author, content, ts_rank(tsv, query) as rank
                             from chat_messages,
                                  to_tsquery($1) as query
                             where tsv @@ query
                             order by rank desc
                             limit $2", &[&query, &limit]).unwrap();
        Ok(rows.iter().map(|r| Match {
            user: r.get("author"),
            message: r.get("content"),
            rank: r.get("rank"),
        }).collect())
    }
}
