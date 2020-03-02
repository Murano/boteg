use boteg::{Message, ResponseMessage};
use failure::Fallible;

#[tokio::main]
async fn main() -> Fallible<()> {
    let mut bot = boteg::Bot::new(([0, 0, 0, 0], 8088), "https://api.telegram.org/bot684490980:AAFLmComOWytWMops7yw4G-MOaIHY0rzpc8/sendMessage");

    bot.add_command("test", |message: Message| {
        let text = format!("test -- {}", message.text);
        Box::pin(async move {
            ResponseMessage {
                chat_id: message.chat.id,
                text,
                parse_mode: None,
            }
        })
    });

    bot.add_command("test2", |message: Message| {
        Box::pin(async move {
            let text = format!("test2 -- {}", message.text);
            ResponseMessage {
                chat_id: message.chat.id,
                text,
                parse_mode: None,
            }
        })
    });

    bot.run().await?;

    Ok(())
}
