
type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

#[tokio::main]
async fn main() -> Result<()> {
    let mut bot = boteg::Bot::new();

    bot.add_command("test", |message|async {
        dbg!(message);
        1
    });

    bot.run().await?;

    Ok(())
}