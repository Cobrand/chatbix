use chrono::NaiveDateTime;
use std::collections::HashMap;
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use std::borrow::Borrow;

use chrono::Duration;
use postgres::Connection as PgConnection;
use rand::{thread_rng, Rng};

use error::*;

#[derive(Debug, Serialize, Clone)]
pub struct ConnectedUser {
    pub username: Arc<String>,
    pub logged_in: bool,
    pub last_active: NaiveDateTime,
    pub last_answer: NaiveDateTime,
}

#[derive(Debug)]
pub struct ConnectedUsers {
    users: HashMap<Arc<String>,ConnectedUser>,
    expiration_time: Duration,
}

impl ConnectedUsers {
    pub fn new() -> ConnectedUsers {
        ConnectedUsers {
            users: HashMap::with_capacity(8),
            expiration_time: Duration::seconds(30),
        }
    }

    pub fn refresh(&mut self) {
        let now = ::chrono::UTC::now().naive_utc();
        let expiration_time = self.expiration_time;
        let users = self.users.drain().filter(|&(_,ref user)|{
            user.last_answer < now + expiration_time
        }).collect::<HashMap<Arc<String>,ConnectedUser>>();
        // ^ TODO: See if this is optimised: (probably not)
        // There are probably better ways to filter values in a hashmap
        self.users = users;
    }

    pub fn update(&mut self, username: &str, logged_in: bool, active: bool) {
        let now = ::chrono::UTC::now().naive_utc();
        let push: bool = {
            let username = String::from(username);
            // ^TODO: file an issue to make HashMap borrow
            // twice so that get("example") when the key is
            // Arc<String> or Rc<String> works, allowing us to avoid a useless allocation
            match self.users.get_mut(&username) {
                Some(mut c) => {
                    if active {
                        c.last_active = now;
                    };
                    c.last_answer = now;
                    false
                },
                None => true
            }
        };
        if push {
            let username = Arc::new(String::from(username));
            self.users.insert(username.clone(), ConnectedUser {
                username: username,
                logged_in: logged_in,
                last_active: now,
                last_answer: now
            }).expect("Unreachable path: key should not exist");
        };
    }

    pub fn as_vec(&self) -> Vec<ConnectedUser> {
        self.users.iter().map(|(_,u)| u.clone()).collect::<Vec<ConnectedUser>>()
    }
}

pub enum UserConnectionStatus {
    AuthFailed,
    NotLoggedIn,
    Connected(bool) //< whether admin or not
}

#[derive(Debug)]
pub struct CachedUser {
    pub auth_key: String,
    pub admin: bool,
}

pub struct CachedUsers(HashMap<String, CachedUser>);

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
