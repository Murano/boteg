use failure::{bail, err_msg, format_err, Fallible};
use std::{future::Future, sync::Arc};

mod messages;
pub use crate::messages::{
    CallbackData, Contents, InlineKeyboardButton, InlineKeyboardMarkup, Message, ResponseMessage,
    Update,
};
use std::{
    net::SocketAddr,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};

mod sender;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
#[cfg(feature = "tls")]
use axum_server::tls_rustls::RustlsConfig;
use sender::Sender;
use std::collections::HashMap;
#[cfg(feature = "tls")]
use std::path::PathBuf;

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
    pub fn new<A: Into<SocketAddr>, U: AsRef<str>>(addr: A, tg_api_uri: U) -> Fallible<Self> {
        Ok(Self {
            commands: vec![],
            current_command: AtomicUsize::new(0),
            enabled_current_command: false,
            callbacks: HashMap::new(),
            addr: addr.into(),
            sender: Sender::new(tg_api_uri.as_ref().parse()?),
        })
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

    #[cfg(feature = "tls")]
    pub async fn run(self) -> Fallible<()> {
        let addr = self.addr;
        let bot = Arc::new(self);

        // configure certificate and private key used by https
        let config = RustlsConfig::from_pem_file(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("serts")
                .join("cert.pem"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("serts")
                .join("key.pem"),
        )
        .await?;

        let app = Router::new().route("/", post(handle)).with_state(bot);
        axum_server::bind_rustls(addr, config)
            .serve(app.into_make_service())
            .await?;

        Ok(())
    }

    #[cfg(not(feature = "tls"))]
    pub async fn run(self) -> Fallible<()> {
        let addr = self.addr;
        let bot = Arc::new(self);

        let app = Router::new().route("/", post(handle)).with_state(bot);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn handle(
    State(bot): State<Arc<Bot>>,
    Json(update): Json<Update>,
) -> Result<Json<()>, StatusCode> {
    let chat_id = update.chat_id().expect("Expecting chat_id");
    let body = dispatch(bot.clone(), update).await.unwrap_or_else(|_err| {
        //TODO log
        ResponseMessage {
            chat_id,
            text: "Got error".to_string(),
            parse_mode: None,
            reply_markup: None,
        }
    });
    if bot.sender.send_message(body).await.is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(()))
}

async fn dispatch(bot: Arc<Bot>, update: Update) -> Fallible<ResponseMessage> {
    let command_idx = bot.current_command.load(Ordering::Relaxed); //TODO не во всех коммандах используется, вынести
    let current_command: &Command = bot.commands.get(command_idx).unwrap();
    let chat_id = update.chat_id();

    let body = match update.contents {
        Contents::CallbackMessage(callback_message) => {
            match bot.callbacks.get(callback_message.data.command.as_str()) {
                Some(cb) => {
                    (cb)(callback_message.message, callback_message.data.message_id).await?
                }
                None => ResponseMessage {
                    chat_id: chat_id.unwrap(),
                    text: callback_message.data.command.clone(),
                    parse_mode: None,
                    reply_markup: None,
                },
            }
        }
        Contents::Current(chat_id) if bot.enabled_current_command => ResponseMessage {
            chat_id,
            text: current_command.name.to_owned(),
            parse_mode: None,
            reply_markup: None,
        },
        Contents::Current(_) => return Err(err_msg("Command current is disabled")),
        Contents::Command(command) => {
            let idx = bot
                .commands
                .iter()
                .position(|existing| existing.name == command.command)
                .ok_or_else(|| format_err!("Command with name: {} not found", command.command))?;
            bot.current_command.store(idx, Ordering::Relaxed);

            let text = format!("Command set to {}", command.command);
            ResponseMessage {
                chat_id: command.chat_id,
                text,
                parse_mode: None,
                reply_markup: None,
            }
        }
        Contents::Message(message) => (current_command.cb)(message).await?,
        Contents::None => bail!("Contents::NONE"),
    };

    Ok(body)
}

struct Command {
    name: &'static str,
    cb: CommandFn,
}
