use crate::{Fallible, ResponseMessage};
use reqwest::{Client, Url};
use serde::Deserialize;

const TG_URL: &str = "https://api.telegram.org";

const SEND_MESSAGE: &str = "sendMessage";

pub struct Sender {
    token: String,
    client: Client,
}

impl Sender {
    pub fn new(token: String) -> Self {
        Self {
            token,
            client: Client::new(),
        }
    }

    pub async fn send_message(&self, message: ResponseMessage) -> Fallible<BotResponse> {
        let uri = Url::parse(&format!("{}/{}/{}", TG_URL, self.token, SEND_MESSAGE))?;

        let result = self.client.post(uri).json(&message).send().await?;

        Ok(result.json().await?)
    }
}

#[derive(Debug, Deserialize)]
pub struct BotResponse {
    pub ok: bool,
}
