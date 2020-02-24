use std::convert::Infallible;

use failure::Fallible;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode, Uri};
use std::sync::Arc;
use std::future::Future;
use bytes::buf::ext::BufExt;

mod messages;
pub use crate::messages::{Update, Contents, ResponseMessage, Message};
use std::net::SocketAddr;
use std::convert::TryInto;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::pin::Pin;

type CommandRef = AtomicUsize;

pub struct Bot<F> {
    commands: Vec<Command<F>>,
    current_command: CommandRef,
    callbacks: Vec<Callback>,
    tg_api_uri: Uri,
    addr: SocketAddr,
}

impl <F, Fut> Bot<F>
    where F: Fn(Message) -> Fut + Send + Sync + 'static,
        Fut: Future<Output=ResponseMessage> + Send + 'static
{

    pub fn new<A: Into<SocketAddr>>(addr: A, tg_api_uri: &'static str) -> Self {
        Self {
            commands: vec![],
            current_command: AtomicUsize::new(0),
            callbacks: vec![],
            tg_api_uri: Uri::from_static(tg_api_uri),
            addr: addr.into(),
        }
    }

    pub fn add_command(&mut self, name: &'static  str, cb: F) {
        self.commands.push(Command {
            name,
            cb: Box::new(cb)
        })
    }

    pub fn add_callback() {

    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>{
        let addr = self.addr;
        let bot = Arc::new(self);
        let make_svc = make_service_fn(move|_conn| {
            let bot = bot.clone();
            async { Ok::<_, Infallible>(service_fn(move|request|{
                let bot = bot.clone();
                bot.handle(request)
            })) }
        });

        let server = Server::bind(&addr).serve(make_svc);
        server.await?;

        Ok(())
    }

    async fn handle(self: Arc<Bot<F>>, request: Request<Body>) -> Fallible<Response<Body>> {
        match (request.method(), request.uri().path()) {
            (&Method::POST, "/bot") => { //FIXME test purpose
                let command_idx = self.current_command.load(Ordering::Relaxed);
                let command = self.commands.get(command_idx).unwrap();
                let whole_body = hyper::body::aggregate(request).await?;
                let update: Update = serde_json::from_reader(whole_body.reader()).unwrap();

                let body = match update.contents {
                    Contents::Command(command) => Body::empty(),
                    Contents::Message(message) => {
                        let response =  (command.cb)(message).await;
                        response.try_into()?
                    },
                    Contents::None => Body::empty()
                };



                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(body)
                    .unwrap()
                )
            },
            _ => {
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap()
                )
            }
        }
    }
}

struct Command<F> {
    name: &'static str,
    cb: Box<F>
}

struct Callback;


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn add_command() {
        let mut bot = Bot::new();

        bot.add_command("test", |message|async{
            println!("It works");
            1
        });

    }
}
