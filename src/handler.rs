
use std::env;
use r2d2_postgres::PostgresConnectionManager;
use chatbix::*;
use std::sync::Arc;
use iron::{mime,Iron,Chain,Response,Request,IronResult,IronError,Set,self};
use mount::Mount;
use router::Router;
use super::routes;
use persistent::Read as PerRead;

extern crate bodyparser;

macro_rules! chatbix_route {
    ($url:tt, $route:path, $arc:expr, $api_handler:expr) => {
        let tmp_chatbix_arc = $arc.clone();
        $api_handler.get($url,
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
        let json_mime : mime::Mime = mime::Mime(mime::TopLevel::Application, mime::SubLevel::Json,
                                                vec![(mime::Attr::Charset,mime::Value::Utf8)]);
        Err(IronError{
            response:err.response.set(json_mime),
            error:err.error,        
        })
    }
}

pub fn handler<C>(chatbix: Chatbix<C>) where Chatbix<C>: ChatbixInterface, C: Send + Sync + 'static {
    let chatbix_arc = Arc::new(chatbix);
    let mut mount = Mount::new();
    let mut api_handler = Router::new();
    
    chatbix_route!("get_messages",routes::get_messages, chatbix_arc, api_handler);
    let mut api_handler = Chain::new(api_handler);
    api_handler.link_before(PerRead::<bodyparser::MaxBodyLength>::one(1024 * 1024)); // limit size of requests to 1MB
    api_handler.link_after(ChatbixAfterMiddleware);
    mount.mount("/api", api_handler);
    let listen_url = env::var("LISTEN_URL").unwrap_or("0.0.0.0:8080".to_owned());
    let _listening = Iron::new(mount).http(&*listen_url).unwrap();
}
