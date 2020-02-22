use std::convert::Infallible;

use failure::Fallible;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::sync::Arc;
use std::future::Future;
use bytes::buf::ext::BufExt;

mod messages;
use messages::Message;
use crate::messages::{Update, Contents};

type CommandRef = usize;

pub struct Bot<F> {
    commands: Vec<Command<F>>,
    current_command: Option<CommandRef>,
    callbacks: Vec<Callback>,
}

impl <F, Fut> Bot<F>
    where F: Fn(Message) -> Fut + Send + Sync + 'static,
        Fut: Future<Output=u8> + Send + 'static
{

    pub fn new() -> Self {
        Self {
            commands: vec![],
            current_command: None,
            callbacks: vec![],
        }
    }

    pub fn add_command(&mut self, name: &'static  str, cb: F) {
        self.commands.push(Command {
            name,
            cb
        })
    }

    pub fn add_callback() {

    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>{
        let bot = Arc::new(self);
        // let bot = bot.clone();
        let make_svc = make_service_fn(move|_conn| {
            // This is the `Service` that will handle the connection.
            // `service_fn` is a helper to convert a function that
            // returns a Response into a `Service`.
            let bot = bot.clone();
            async { Ok::<_, Infallible>(service_fn(move|request|{
                let bot = bot.clone();
                bot.handle(request)
            })) }
        });

        let addr = ([0, 0, 0, 0], 8088).into();

        let server = Server::bind(&addr).serve(make_svc);

        println!("Listening on http://{}", addr);

        server.await?;

        Ok(())
    }

    async fn handle(self: Arc<Bot<F>>, request: Request<Body>) -> Fallible<Response<Body>> {
        match (request.method(), request.uri().path()) {
            (&Method::POST, "/bot") => { //FIXME test purpose
                let command_idx = self.current_command.unwrap_or_default();
                let command = self.commands.get(command_idx).unwrap();
                let whole_body = hyper::body::aggregate(request).await?;
                let update: Update = serde_json::from_reader(whole_body.reader()).unwrap();

                match update.contents {
                    Contents::Command(_) => unimplemented!(),
                    Contents::Message(message) => (command.cb)(message).await,
                    Contents::None => unimplemented!()
                };

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
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
    cb: F
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
