use chrono::NaiveDateTime;
use std::collections::HashMap;

use postgres::Connection as PgConnection;
use rand::{thread_rng, Rng};

use error::*;

#[derive(Debug, Serialize, Clone)]
pub struct ConnectedUser {
    pub username: String,
    pub logged_in: bool,
    pub last_active: NaiveDateTime,
    pub last_answer: NaiveDateTime,
}

pub enum UserConnectionStatus {
    AuthFailed,
    NotLoggedIn,
    Connected(bool) //< whether admin or not
}

pub struct CachedUser {
    pub auth_key: String,
    pub admin: bool,
}

pub struct CachedUsers(HashMap<String,CachedUser>); 

impl CachedUsers {
    pub fn new() -> CachedUsers {
        CachedUsers(HashMap::new())
    }

    pub fn login(&mut self, username: &str, admin: bool) -> String {
        if self.0.contains_key(username) {
            self.0.get(username).unwrap().auth_key.clone()
        } else {
            // generate new key
            let auth_key : String = thread_rng().gen_ascii_chars().take(16).collect();
            self.0.insert(username.to_owned(), CachedUser {auth_key: auth_key.clone(), admin: admin});
            auth_key.clone()
        }
    }

    // Maybe return a Result here?
    // return false if not connected
    pub fn logout(&mut self, username: &str, auth_key: &str) -> Result<()> {
        match self.0.get(username) {
            Some(c) => {
                if c.auth_key != auth_key {
                    bail!(ErrorKind::InvalidAuthKey)
                };
            },
            None => bail!(ErrorKind::NotLoggedIn)
        };
        self.0.remove(username);
        Ok(())
    }
    
    pub fn check(&self, username: &str, auth_key: &str) -> UserConnectionStatus {
        match self.0.get(username) {
            Some(cached_user) => {
                match (cached_user.auth_key == auth_key, cached_user.admin) {
                    (false,_) => UserConnectionStatus::AuthFailed,
                    (true,false) => UserConnectionStatus::Connected(false),
                    (true,true) => UserConnectionStatus::Connected(true),
                }
            },
            None => UserConnectionStatus::NotLoggedIn,
        }
    }
}
