
use std::env;
use r2d2_postgres::PostgresConnectionManager;
use chatbix::*;
use std::sync::Arc;
use iron::{status,mime,Iron,Chain,Response,Request,IronResult,IronError,Set,self};
use mount::Mount;
use router::Router;
use super::routes;
use persistent::Read as PerRead;
use std::io::Write as IoWrite;
use std::thread;
use std::sync::mpsc;
use std::time::Duration;

extern crate bodyparser;

macro_rules! chatbix_route {
    ($method:ident,$url:tt, $route:path, $arc:expr, $api_handler:expr) => {
        let tmp_chatbix_arc = $arc.clone();
        $api_handler.$method($url,
                        move |r: &mut Request| {
                            let chatbix_arc = tmp_chatbix_arc.clone();
                            ($route)(r, chatbix_arc)
                        },
                        $url)
    }
}

struct ChatbixAfterMiddleware;

impl iron::AfterMiddleware for ChatbixAfterMiddleware {
    fn after(&self, r: &mut Request, res: Response) -> IronResult<Response> {
        let json_mime : mime::Mime = mime::Mime(mime::TopLevel::Application, mime::SubLevel::Json,
                                                vec![(mime::Attr::Charset,mime::Value::Utf8)]);
        Ok(res.set(json_mime))
    }

    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        use std::fmt::Write;
        let json_mime : mime::Mime = mime::Mime(mime::TopLevel::Application, mime::SubLevel::Json,
                                                vec![(mime::Attr::Charset,mime::Value::Utf8)]);
        let mut answer : Vec<u8> = Vec::new();
        let status_code = err.response.status.unwrap_or(status::InternalServerError);
        if status_code.is_server_error() {
            println!("Unexpected {1} error: `{0}` ({0:?})", err, status_code);
        }

        // this part is to allow any error to be translated JSON style.
        write!(&mut answer,r#"{{"error":""#);
        match err.response.body {
            Some(mut b) => b.write_body(&mut answer),
            None => write!(&mut answer,"{}",err.error),
        };
        write!(&mut answer,r#""}}"#);
        Ok(Response::with((status_code,answer,json_mime)))
    }
}

pub fn handler<C>(chatbix: Chatbix<C>) where Chatbix<C>: ChatbixInterface, C: Send + Sync + 'static {
    let chatbix_arc = Arc::new(chatbix);
    let mut mount = Mount::new();
    let mut api_handler = Router::new();
    let chatbix_weak = Arc::downgrade(&chatbix_arc);
    thread::spawn(move || {
        while let Some(chatbix_arc) = chatbix_weak.upgrade() {
            chatbix_arc.refresh_users();
            // wait 2 seconds to filter connected users
            thread::sleep(Duration::new(2,0));
        }
        // Stop when there are no more Arc<Chatbix<_>> active
    });
    chatbix_route!(get,"get_messages",routes::get_messages, chatbix_arc, api_handler);
    chatbix_route!(post,"new_message",routes::new_message, chatbix_arc, api_handler);
    chatbix_route!(post,"login",routes::login, chatbix_arc, api_handler);
    chatbix_route!(post,"logout",routes::logout, chatbix_arc, api_handler);
    chatbix_route!(post,"register",routes::register, chatbix_arc, api_handler);
    chatbix_route!(get,"heartbeat",routes::heartbeat, chatbix_arc, api_handler);
    let mut api_handler = Chain::new(api_handler);
    api_handler.link_before(PerRead::<bodyparser::MaxBodyLength>::one(1024 * 1024)); // limit size of requests to 1MB
    api_handler.link_after(ChatbixAfterMiddleware);
    mount.mount("/api", api_handler);
    let listen_url = env::var("LISTEN_URL").unwrap_or("0.0.0.0:8080".to_owned());
    let _listening = Iron::new(mount).http(&*listen_url).unwrap();
}
