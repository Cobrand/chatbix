use chrono::NaiveDateTime;
use std::collections::HashMap;

use postgres::Connection as PgConnection;

pub struct ConnectedUser {
    pub uid: u64,
    pub username: String,
    pub logged_in: bool,
    pub admin: bool,
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

    // Maybe return a Result here ?
    pub fn login_pg(&mut self, pg: &PgConnection, username: &str, password: &str) {
        
    }

    // Maybe return a Result here?
    pub fn logout(&mut self, username: &str, auth_key: &str) {

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
