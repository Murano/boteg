use boteg::Fallible;
use boteg::{CallbackData, InlineKeyboardButton, InlineKeyboardMarkup, Message, ResponseMessage};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Fallible<()> {
    let mut bot = boteg::Bot::new(
        ([0, 0, 0, 0], 8088),
        "bot684490980:AAFLmComOWytWMops7yw4G-MOaIHY0rzpc8".to_owned(),
        Some(get_sert_path("cert.pem")),
        Some(get_sert_path("key.pem")),
    )?;

    bot.add_command_static("test", |message: Message| {
        let text = format!("test -- {}", message.text);

        let reply_markup = InlineKeyboardButton {
            text: "Exchanges".to_string(),
            callback_data: CallbackData {
                command: "exchanges".to_string(),
                message_id: Some(message.message_id),
            },
        };

        let keyboard = InlineKeyboardMarkup {
            inline_keyboard: [vec![reply_markup]],
        };

        Box::pin(async move {
            Ok(ResponseMessage {
                chat_id: message.chat.id,
                text,
                parse_mode: None,
                reply_markup: Some(keyboard),
            })
        })
    });

    let mut button_state = true;

    bot.add_callback_static("exchanges", |message: Message, message_id: Option<u64>| {
        let text = "Select exchange".to_owned();
        dbg!(&message);

        let reply_markup = InlineKeyboardButton {
            text: "Bybit".to_string(),
            callback_data: CallbackData {
                command: "exchanges".to_string(),
                message_id,
            },
        };

        let reply_markup2 = InlineKeyboardButton {
            text: "Kucoin".to_string(),
            callback_data: CallbackData {
                command: "exchanges".to_string(),
                message_id: None,
            },
        };

        let keyboard = InlineKeyboardMarkup {
            inline_keyboard: [vec![reply_markup, reply_markup2]],
        };

        Box::pin(async move {
            Ok(ResponseMessage {
                chat_id: message.chat.id,
                text,
                parse_mode: None,
                reply_markup: Some(keyboard),
            })
        })
    });

    bot.add_callback_static("test2", |message: Message, message_id: Option<u64>| {
        let text = format!("test2 callback -- {} - {:?}", message.text, message_id);

        let reply_markup = InlineKeyboardButton {
            text: "More examples".to_string(),
            callback_data: CallbackData {
                command: "more".to_string(),
                message_id,
            },
        };

        let reply_markup2 = InlineKeyboardButton {
            text: "Other".to_string(),
            callback_data: CallbackData {
                command: "test2".to_string(),
                message_id: None,
            },
        };

        let keyboard = InlineKeyboardMarkup {
            inline_keyboard: [vec![reply_markup, reply_markup2]],
        };

        Box::pin(async move {
            Ok(ResponseMessage {
                chat_id: message.chat.id,
                text,
                parse_mode: None,
                reply_markup: Some(keyboard),
            })
        })
    });

    bot.add_command_inline_static("test2", |message: Message| {
        Box::pin(async move {
            let text = format!("test2 -- {}", message.text);

            let reply_markup = InlineKeyboardButton {
                text: "More examples".to_string(),
                callback_data: CallbackData {
                    command: "test2".to_string(),
                    message_id: Some(message.message_id),
                },
            };

            let keyboard = InlineKeyboardMarkup {
                inline_keyboard: [vec![reply_markup]],
            };

            Ok(ResponseMessage {
                chat_id: message.chat.id,
                text,
                parse_mode: None,
                reply_markup: Some(keyboard),
            })
        })
    });

    bot.enable_current_command();

    bot.run().await?;

    Ok(())
}

fn get_sert_path(sert: &str) -> PathBuf {
    PathBuf::new()
        .join(env!("CARGO_MANIFEST_DIR"))
        .join("certs")
        .join(sert)
}
