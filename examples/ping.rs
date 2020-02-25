use boteg::{ResponseMessage, Message};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

#[tokio::main]
async fn main() -> Result<()> {
    let mut bot = boteg::Bot::new(([0, 0, 0, 0], 8088), "http://dfgdfgdfg.ru");

    bot.add_command("test", |message : Message|{
        let text = format!("test -- {}", message.text);
        Box::pin(async move {

            ResponseMessage {
                chat_id: message.chat.id,
                text,
                parse_mode: None
            }
        })

    });

    bot.add_command("test2", |message : Message| Box::pin(async move {
        let text = format!("test2 -- {}", message.text);
        ResponseMessage {
            chat_id: message.chat.id,
            text,
            parse_mode: None
        }
    }));

    bot.run().await?;

    Ok(())
}