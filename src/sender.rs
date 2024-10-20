use crate::ResponseMessage;
use failure::Fallible;
use reqwest::{Client, Url};

pub struct Sender {
    uri: Url,
    client: Client,
}

impl Sender {
    pub fn new(uri: Url) -> Self {
        Self {
            uri,
            client: Client::new(),
        }
    }

    pub async fn send_message(&self, message: ResponseMessage) -> Fallible<()> {
        self.client
            .post(self.uri.clone())
            .json(&message)
            .send()
            .await?;
        Ok(())
    }
}
