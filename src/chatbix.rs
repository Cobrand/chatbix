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


pub trait ChatbixInterface {
    type InitParams;
    fn new(init_params: Self::InitParams) -> Self;

    fn get_messages<V: AsRef<[String]>>(&self, timestamp: Option<NaiveDateTime>, timestamp_end: Option<NaiveDateTime>, channels: V, include_default_channel: bool) -> Result<Vec<Message>>;

    fn new_message(&self, new_message: &NewMessage) -> Result<()>;

    /// returns some auth_key
    fn register(&self, username: &str, password: &str) -> Result<String>;

    /// return auth_key
    fn login(&self, username: &str, password: &str) -> Result<String>;
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
        let pg : PooledConnection<_> = try!(self.connection.get().map_err(|_| Error::from_kind(ErrorKind::DatabaseBusy)));
        let timestamp : NaiveDateTime = now();
        let mut tags : i32= new_message.tags.unwrap_or(0) & 0b000_0000_0000_0000_0000_0000_1111_1110i32; // see User.tags for more info
        if let Some(ref auth_key) = new_message.auth_key {
            let cached_users = self.cached_users.read().unwrap();
            match cached_users.check(&*new_message.author, &*auth_key) {
                UserConnectionStatus::NotLoggedIn => bail!(ErrorKind::NotLoggedIn),
                UserConnectionStatus::AuthFailed => bail!(ErrorKind::InvalidAuthKey),
                UserConnectionStatus::Connected(_) => {
                    tags |= 1;
                }
            }
        };
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
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp > $1 AND (channel IS NULL OR channel = ANY ($2)) ORDER BY timestamp ASC;",
                             &[&timestamp,&channels.as_ref()]).unwrap()
                } else {
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp > $1 AND channel = ANY ($2) ORDER BY timestamp ASC;",
                             &[&timestamp,&channels.as_ref()]).unwrap()
                },
            (Some(timestamp),Some(timestamp_end)) =>
                if include_default_channel {
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp > $1 AND chat_messages.timestamp < $2 AND (channel IS NULL OR channel = ANY ($3)) ORDER BY timestamp ASC;",
                             &[&timestamp,&timestamp_end,&channels.as_ref()]).unwrap()
                } else { 
                    pg.query("SELECT * FROM chat_messages WHERE chat_messages.timestamp > $1 AND chat_messages.timestamp < $2 AND channel = ANY ($3) ORDER BY timestamp ASC;",
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
}
