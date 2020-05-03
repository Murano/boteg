use std::convert::Infallible;

use bytes::buf::ext::BufExt;
use failure::{err_msg, format_err, Fallible};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode, Uri,
};
use std::{future::Future, sync::Arc};

mod messages;
pub use crate::messages::{
    CallbackData, Contents, InlineKeyboardButton, InlineKeyboardMarkup, Message,
    ResponseMessage, Update,
};
use std::{
    convert::TryInto,
    net::SocketAddr,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};

mod sender;
use sender::Sender;
use std::collections::HashMap;

type CommandRef = AtomicUsize;
type Fut = Pin<Box<dyn Future<Output = Fallible<ResponseMessage>> + Send + 'static>>;
type CallbackCommandFn = Box<dyn Fn(Message, Option<u64>) -> Fut + Send + Sync + 'static>;
type CommandFn = Box<dyn Fn(Message) -> Fut + Send + Sync + 'static>;

pub struct Bot {
    commands: Vec<Command>,
    current_command: CommandRef,
    enabled_current_command: bool,
    callbacks: HashMap<&'static str, CallbackCommandFn>,
    addr: SocketAddr,
    sender: Sender,
}

impl Bot {
    pub fn new<A: Into<SocketAddr>>(addr: A, tg_api_uri: &'static str) -> Self {
        Self {
            commands: vec![],
            current_command: AtomicUsize::new(0),
            enabled_current_command: false,
            callbacks: HashMap::new(),
            addr: addr.into(),
            sender: Sender::new(Uri::from_static(tg_api_uri)),
        }
    }

    pub fn add_command<F: Fn(Message) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: &'static str,
        cb: F,
    ) {
        if self.commands.iter().any(|command| command.name == name) {
            panic!("Command with name: `{}` already exists", name);
        }

        self.commands.push(Command {
            name,
            cb: Box::new(cb),
        });
    }

    pub fn enable_current_command(&mut self) {
        self.enabled_current_command = true;
    }

    pub fn add_callback<F: Fn(Message, Option<u64>) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: &'static str,
        cb: F,
    ) {
        self.callbacks.insert(name, Box::new(cb));
    }

    pub async fn run(self) -> Fallible<()> {
        let addr = self.addr;
        let bot = Arc::new(self);
        let make_svc = make_service_fn(move |_conn| {
            let bot = bot.clone();
            async {
                Ok::<_, Infallible>(service_fn(move |request| {
                    let bot = bot.clone();
                    bot.handle(request)
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);
        server.await?;

        Ok(())
    }

    async fn handle(self: Arc<Bot>, request: Request<Body>) -> Fallible<Response<Body>> {
        if let Method::POST = *request.method() {
            let whole_body = hyper::body::aggregate(request).await?;
            let update: Update = serde_json::from_reader(whole_body.reader()).unwrap();
            let chat_id = update.chat_id().expect("Expecting chat_id");

            let bot = Arc::clone(&self);
            let body = match dispatch(bot, update).await {
                Ok(body) => body,
                Err(_err) => {
                    //TODO log
                    ResponseMessage {
                        chat_id,
                        text: "Got error".to_string(),
                        parse_mode: None,
                        reply_markup: None,
                    }
                    .try_into()?
                },
            };
            self.sender.send_message(body).await?;
        }

        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap())
    }
}

async fn dispatch(bot: Arc<Bot>, update: Update) -> Fallible<Body> {
    let command_idx = bot.current_command.load(Ordering::Relaxed); //TODO не во всех коммандах используется, вынести
    let current_command: &Command = bot.commands.get(command_idx).unwrap();
    let chat_id = update.chat_id();

    let body = match update.contents {
        Contents::CallbackMessage(callback_message) => {
            match bot.callbacks.get(callback_message.data.command.as_str()) {
                Some(cb) => {
                    let response =
                        (cb)(callback_message.message, callback_message.data.message_id)
                            .await?;
                    response.try_into()?
                },
                None => ResponseMessage {
                    chat_id: chat_id.unwrap(),
                    text: callback_message.data.command.clone(),
                    parse_mode: None,
                    reply_markup: None,
                }
                .try_into()?,
            }
        },
        Contents::Current(chat_id) if bot.enabled_current_command => ResponseMessage {
            chat_id,
            text: current_command.name.to_owned(),
            parse_mode: None,
            reply_markup: None,
        }
        .try_into()?,
        Contents::Current(_) => return Err(err_msg("Command current is disabled")),
        Contents::Command(command) => {
            let idx = bot
                .commands
                .iter()
                .position(|existing| existing.name == command.command)
                .ok_or_else(|| {
                    format_err!("Command with name: {} not found", command.command)
                })?;
            bot.current_command.store(idx, Ordering::Relaxed);

            let text = format!("Command set to {}", command.command);
            ResponseMessage {
                chat_id: command.chat_id,
                text,
                parse_mode: None,
                reply_markup: None,
            }
            .try_into()?
        },
        Contents::Message(message) => {
            let response = (current_command.cb)(message).await?;
            response.try_into()?
        },
        Contents::None => Body::empty(),
    };

    Ok(body)
}

struct Command {
    name: &'static str,
    cb: CommandFn,
}

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

        bot.add_command("test", |message| {
            async {
                println!("It works");
                1
            }
        });
    }
}
