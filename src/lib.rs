#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;

extern crate r2d2;
extern crate r2d2_postgres;
extern crate postgres;

extern crate dotenv;
extern crate chrono;

extern crate iron;
extern crate staticfile;
extern crate mount;
extern crate router;
extern crate bodyparser;
extern crate urlencoded;
extern crate persistent;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod error;
mod message;
mod chatbix;
mod user;
mod routes;
mod handler;

use dotenv::dotenv;
use std::env;
use std::sync::Arc;

use message::Message;
use r2d2_postgres::{TlsMode, PostgresConnectionManager};

use chatbix::*;

use error::*;

pub fn run_pg() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let manager = PostgresConnectionManager::new(database_url,TlsMode::None).expect("Failed to establish connection to postgres instance");
    let pg_pool = r2d2::Pool::new(r2d2::Config::default(),manager).unwrap();
    let chatbix = Chatbix::new(pg_pool);
    handler::handler(chatbix);
}
