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

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod error;
mod message;
mod chatbix;
mod user;
mod routes;

use dotenv::dotenv;
use std::env;

use message::Message;
use r2d2_postgres::{TlsMode, PostgresConnectionManager};

use std::sync::Arc;
use chatbix::*;

use error::*;
use iron::{Iron,Response,Request};
use mount::Mount;
use router::Router;

pub fn run_pg() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let listen_url = env::var("LISTEN_URL")
        .unwrap_or("0.0.0.0:8080".to_owned());
    let manager = PostgresConnectionManager::new(database_url,TlsMode::None).expect("Failed to establish connection to postgres instance");
    let pg_pool = r2d2::Pool::new(r2d2::Config::default(),manager).unwrap();
    /* let rows = pg.query("SELECT * FROM (SELECT * FROM chat_messages ORDER BY timestamp DESC LIMIT 5) as pote ORDER BY timestamp ASC;",&[]).unwrap();
    for row in rows.into_iter() {
        let author : String = row.get("author");
        let content : String = row.get("content");
        println!("{}: {}", author, content);
    } */
    let chatbix_arc = Arc::new(Chatbix::new(pg_pool));
    let mut mount = Mount::new();
    let mut api_handler = Router::new();
    let tmp_chatbix_arc = chatbix_arc.clone(); // will be moved in closure
    api_handler.get("get_messages",
                    move |r: &mut Request| {
                        let chatbix_arc = tmp_chatbix_arc.clone();
                        routes::get_messages(r, chatbix_arc)
                    },
                    "get_messages");
    mount.mount("/api", api_handler);

    let listening = Iron::new(mount).http(&*listen_url).unwrap();
}
