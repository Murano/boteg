use boteg::{ResponseMessage, Message};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

#[tokio::main]
async fn main() -> Result<()> {
    let mut bot = boteg::Bot::new(([0, 0, 0, 0], 8088), "http://dfgdfgdfg.ru");

    bot.add_command("test", |message: Message| async move {

        ResponseMessage {
            chat_id: message.chat.id,
            text: message.text,
            parse_mode: None
        }
    });

   /* bot.add_command("test2", |message : Message| async move {

        ResponseMessage {
            chat_id: message.chat.id,
            text: message.text,
            parse_mode: None
        }
    });*/

    /*bot.add_command("test3", |message : Message| async move {

        ResponseMessage {
            chat_id: message.chat.id,
            text: message.text,
            parse_mode: None
        }
    });*/

    bot.run().await?;

    Ok(())
}