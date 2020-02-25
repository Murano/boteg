use std::convert::Infallible;

use failure::{Fallible, format_err};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode, Uri};
use std::sync::Arc;
use std::future::Future;
use bytes::buf::ext::BufExt;
use futures_util::future::FutureExt;

mod messages;
pub use crate::messages::{Update, Contents, ResponseMessage, Message};
use std::net::SocketAddr;
use std::convert::TryInto;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::pin::Pin;

mod sender;
use sender::Sender;

type CommandRef = AtomicUsize;
type Fut = Pin<Box<dyn Future<Output=ResponseMessage> + Send + 'static>>;
type CommandFn = Box<dyn Fn(Message) -> Fut + Send + Sync + 'static>;

pub struct Bot {
    commands: Vec<Command>,
    current_command: CommandRef,
    callbacks: Vec<Callback>,
    addr: SocketAddr,
    sender: Sender,
}

impl Bot
{

    pub fn new<A: Into<SocketAddr>>(addr: A, tg_api_uri: &'static str) -> Self {
        Self {
            commands: vec![],
            current_command: AtomicUsize::new(0),
            callbacks: vec![],
            addr: addr.into(),
            sender: Sender::new(Uri::from_static(tg_api_uri))
        }
    }

    pub fn add_command<F: Fn(Message) -> Fut + Send + Sync + 'static>(&mut self, name: &'static  str, cb: F){

        if self.commands.iter()
            .any(|command| command.name == name) {
            panic!("Command with name: `{}` already exists", name);
        }

        self.commands.push(Command {
            name,
            cb: Box::new(cb)
        });
    }

    pub fn add_callback() {

    }

    pub async fn run(self) -> Fallible<()>{
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

    async fn handle(self: Arc<Bot>, request: Request<Body>) -> Fallible<Response<Body>> {
        match request.method() {
            &Method::POST => {

                let whole_body = hyper::body::aggregate(request).await?;
                let update: Update = serde_json::from_reader(whole_body.reader()).unwrap();
                let chat_id = update.chat_id().expect("Expecting chat_id");

                let bot = Arc::clone(&self);
                let body = match dispatch(bot, update).await {
                    Ok(body) => body,
                    Err(err) => {
                        //TODO log
                        ResponseMessage {
                            chat_id,
                            text: "Got error".to_string(),
                            parse_mode: None
                        }.try_into()?
                    }
                };
                self.sender.send_message(body).await?;
            },
            _ => {}
        };
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap())
    }
}

async fn dispatch(bot: Arc<Bot>, update: Update) -> Fallible<Body> {
    let command_idx = bot.current_command.load(Ordering::Relaxed);
    let current_command: &Command = bot.commands.get(command_idx).unwrap();

    let body = match update.contents {
        Contents::Command(command) => {

            let idx = bot.commands.iter()
                .position(|existing|existing.name == command.command)
                .ok_or_else(||format_err!("Command with name: {} not found", command.command))?;
            bot.current_command.store(idx, Ordering::Relaxed);

            let text = format!("Command set to {}", command.command);
            ResponseMessage {
                chat_id: command.chat_id,
                text,
                parse_mode: None
            }.try_into()?
        },
        Contents::Message(message) => {

            let response = (current_command.cb)(message).await;
            response.try_into()?
        },
        Contents::None => Body::empty()
    };

    Ok(body)
}

struct Command {
    name: &'static str,
    cb: CommandFn
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
