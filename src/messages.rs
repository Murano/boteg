use serde::{de, Deserialize, Serialize, Deserializer};

#[derive(Deserialize)]
pub struct Update {
    pub update_id: i32,
    pub message: Option<Message>
}

#[derive(Debug, Deserialize)]
pub struct Message {
    #[serde(deserialize_with = "deserialize_contents")]
    pub text: Contents,
}

#[derive(Debug, Deserialize)]
pub enum Contents {
    Command(String),
    Text(String)
}

fn deserialize_contents<'de, D>(
    deserializer: D,
) -> Result<Contents, D::Error>
    where
        D: Deserializer<'de>,
{
    let raw_text = String::deserialize(deserializer)?;

    let contents = match raw_text.chars().next() {
        Some('/') => {
            Contents::Command(String::from(&raw_text[1..]))
        },
        _ => Contents::Text(raw_text)
    };
    Ok(contents)
}

#[derive(Deserialize)]
pub struct CallbackMessage;

#[derive(Serialize)]
pub struct ResponseMessage;