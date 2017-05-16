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
extern crate crypto;
extern crate rand;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[macro_use]
mod error;
mod message;
mod chatbix;
mod user;
mod routes;
mod handler;
mod utils;

use dotenv::dotenv;
use std::env;

use r2d2_postgres::{TlsMode, PostgresConnectionManager};

use chatbix::*;

pub fn run_pg() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let manager = PostgresConnectionManager::new(database_url,TlsMode::None).expect("Failed to establish connection to postgres instance");
    let pg_pool_config = r2d2::Config::builder().pool_size(15).min_idle(Some(3)).build();
    let pg_pool = r2d2::Pool::new(pg_pool_config, manager).unwrap();
    let chatbix = Chatbix::new(pg_pool);
    handler::handler(chatbix);
}
