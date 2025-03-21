use anyhow::{bail, format_err};
use std::{future::Future, sync::Arc};

pub type Fallible<T> = anyhow::Result<T>;

mod messages;
pub use crate::messages::{
    CallbackData, Contents, InlineKeyboardButton, InlineKeyboardMarkup, Message, ResponseMessage,
    Update,
};
use std::borrow::Cow;
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
    inline_commands: HashMap<Cow<'static, str>, Command>,
    current_command: CommandRef,
    enabled_current_command: bool,
    callbacks: HashMap<Cow<'static, str>, CallbackCommandFn>,
    addr: SocketAddr,
    sender: Sender,
    #[cfg(feature = "tls")]
    cert: Option<PathBuf>,
    #[cfg(feature = "tls")]
    key: Option<PathBuf>,
}

impl Bot {
    #[cfg(not(feature = "tls"))]
    pub fn new<A: Into<SocketAddr>>(addr: A, token: String) -> Fallible<Self> {
        Ok(Self {
            commands: vec![],
            inline_commands: HashMap::default(),
            current_command: AtomicUsize::new(0),
            enabled_current_command: false,
            callbacks: HashMap::new(),
            addr: addr.into(),
            sender: Sender::new(token),
        })
    }

    #[cfg(feature = "tls")]
    pub fn new<A: Into<SocketAddr>>(
        addr: A,
        token: String,
        cert: Option<PathBuf>,
        key: Option<PathBuf>,
    ) -> Fallible<Self> {
        Ok(Self {
            commands: vec![],
            inline_commands: HashMap::default(),
            current_command: AtomicUsize::new(0),
            enabled_current_command: false,
            callbacks: HashMap::new(),
            addr: addr.into(),
            sender: Sender::new(token),
            cert,
            key,
        })
    }

    pub fn add_command_static<F: Fn(Message) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: &'static str,
        cb: F,
    ) {
        if let Err(err) = self.add_command(Cow::Borrowed(name), cb) {
            panic!("{:?}", err);
        }
    }

    pub fn add_command_dynamic<F: Fn(Message) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: String,
        cb: F,
    ) -> Fallible<()> {
        self.add_command(Cow::Owned(name), cb)
    }

    fn add_command<F: Fn(Message) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: Cow<'static, str>,
        cb: F,
    ) -> Fallible<()> {
        if self.commands.iter().any(|command| command.name == name) {
            bail!("Command with name: `{}` already exists", name);
        }

        self.commands.push(Command {
            name,
            cb: Box::new(cb),
        });
        Ok(())
    }

    pub fn add_command_inline_static<F: Fn(Message) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: &'static str,
        cb: F,
    ) {
        if let Err(err) = self.add_command_inline(Cow::Borrowed(name), cb) {
            panic!("{:?}", err);
        }
    }

    pub fn add_command_inline_dynamic<F: Fn(Message) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: String,
        cb: F,
    ) -> Fallible<()> {
        self.add_command_inline(Cow::Owned(name), cb)
    }

    fn add_command_inline<F: Fn(Message) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: Cow<'static, str>,
        cb: F,
    ) -> Fallible<()> {
        if self.inline_commands.contains_key(&name) {
            bail!("Inline command with name: `{}` already exists", &name);
        }

        self.inline_commands.insert(
            name.clone(),
            Command {
                name,
                cb: Box::new(cb),
            },
        );
        Ok(())
    }

    pub fn enable_current_command(&mut self) {
        self.enabled_current_command = true;
    }

    pub fn add_callback_static<F: Fn(Message, Option<u64>) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: &'static str,
        cb: F,
    ) {
        if let Err(err) = self.add_callback(Cow::Borrowed(name), cb) {
            panic!("{:?}", err);
        }
    }

    pub fn add_callback_dynamic<F: Fn(Message, Option<u64>) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: String,
        cb: F,
    ) -> Fallible<()> {
        self.add_callback(Cow::Owned(name), cb)
    }

    fn add_callback<F: Fn(Message, Option<u64>) -> Fut + Send + Sync + 'static>(
        &mut self,
        name: Cow<'static, str>,
        cb: F,
    ) -> Fallible<()> {
        if self.inline_commands.contains_key(&name) {
            bail!("Inline command with name: `{}` already exists", &name);
        }

        self.callbacks.insert(name, Box::new(cb));
        Ok(())
    }

    #[cfg(feature = "tls")]
    pub async fn run(self) -> Fallible<()> {
        let addr = self.addr;

        let config = match (&self.cert, &self.key) {
            (Some(cert), Some(key)) => {
                RustlsConfig::from_pem_file(PathBuf::from(cert), PathBuf::from(key)).await?
            }
            (_, _) => {
                RustlsConfig::from_pem_file(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("serts")
                        .join("cert.pem"),
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("serts")
                        .join("key.pem"),
                )
                .await?
            }
        };

        let bot = Arc::new(self);

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
            text: "Got error".to_owned(),
            parse_mode: None,
            reply_markup: None,
        }
    });
    match bot.sender.send_message(body).await {
        Ok(response) if response.ok => Ok(Json(())),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn dispatch(bot: Arc<Bot>, update: Update) -> Fallible<ResponseMessage> {
    let chat_id = update.chat_id();

    let body = match update.contents {
        Contents::CallbackMessage(callback_message) => {
            match bot.callbacks.get(callback_message.data.command.as_str()) {
                Some(cb) => {
                    (cb)(callback_message.message, callback_message.data.message_id).await?
                }
                None => ResponseMessage {
                    chat_id: chat_id.unwrap(),
                    text: callback_message.data.command,
                    parse_mode: None,
                    reply_markup: None,
                },
            }
        }
        Contents::Current(chat_id) if bot.enabled_current_command => {
            let command_idx = bot.current_command.load(Ordering::Relaxed);
            let current_command: &Command = bot.commands.get(command_idx).unwrap();
            ResponseMessage {
                chat_id,
                text: current_command.name.clone().into_owned(),
                parse_mode: None,
                reply_markup: None,
            }
        }
        Contents::Current(_) => bail!("Command current is disabled"),
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
        Contents::Message(message) => {
            if let Some(inline_command) = bot.inline_commands.get(message.text.as_str()) {
                (inline_command.cb)(message).await?
            } else {
                let command_idx = bot.current_command.load(Ordering::Relaxed);
                let current_command: &Command = bot.commands.get(command_idx).unwrap();
                (current_command.cb)(message).await?
            }
        }
        Contents::None => bail!("Contents::NONE"),
    };

    Ok(body)
}

struct Command {
    name: Cow<'static, str>,
    cb: CommandFn,
}
