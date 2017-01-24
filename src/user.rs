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

    /// return None -> not connected
    /// return Some(false) -> connected, no admin rights
    /// return Some(true) -> connected with admin rights
    pub fn check(&self, username: &str, auth_key: &str) -> Option<bool> {
        self.0.get(username).and_then(|cached_user| {
            match (cached_user.auth_key == auth_key, cached_user.admin) {
                (false,_) => None,
                (true,false) => Some(false),
                (true,true) => Some(true),
            }
        })
    }
}
