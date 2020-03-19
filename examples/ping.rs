use boteg::{InlineKeyboardButton, InlineKeyboardMarkup, Message, ResponseMessage};
use failure::Fallible;

#[tokio::main]
async fn main() -> Fallible<()> {
    let mut bot = boteg::Bot::new(
        ([0, 0, 0, 0], 8088),
        "https://api.telegram.org/888/sendMessage",
    );

    bot.add_command("test", |message: Message| {
        let text = format!("test -- {}", message.text);

        let reply_markup = InlineKeyboardButton {
            text: "More examples".to_string(),
            callback_data: "more".to_string(),
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

    bot.add_callback("more", |message: Message| {
        let text = format!("more -- {}", message.text);

        let reply_markup = InlineKeyboardButton {
            text: "More examples".to_string(),
            callback_data: "more".to_string(),
        };

        let reply_markup2 = InlineKeyboardButton {
            text: "Other".to_string(),
            callback_data: "other".to_string(),
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

    bot.add_command("test2", |message: Message| {
        Box::pin(async move {
            let text = format!("test2 -- {}", message.text);
            Ok(ResponseMessage {
                chat_id: message.chat.id,
                text,
                parse_mode: None,
                reply_markup: None,
            })
        })
    });

    bot.enable_current_command();

    bot.run().await?;

    Ok(())
}
