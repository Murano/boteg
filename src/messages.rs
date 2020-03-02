use failure::_core::{
    convert::TryFrom,
    fmt::{Debug, Formatter},
};
use hyper::Body;
use serde::{
    de,
    de::{MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use std::fmt;

#[derive(Debug)]
pub struct Update {
    pub update_id: u64,
    pub contents: Contents,
}

impl Update {
    pub fn chat_id(&self) -> Option<u64> {
        match &self.contents {
            Contents::Command(command) => Some(command.chat_id),
            Contents::Message(message) => Some(message.chat.id),
            Contents::None => None,
        }
    }
}

impl<'de> Deserialize<'de> for Update {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct UpdateVisitor;

        impl<'de> Visitor<'de> for UpdateVisitor {
            type Value = Update;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("struct Update")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Update, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut update_id = None;
                let mut contents = None;

                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    match &*key {
                        "update_id" => {
                            if update_id.is_some() {
                                return Err(de::Error::duplicate_field("update_id"));
                            }
                            update_id = value.as_u64();
                        },
                        "message" => {
                            if contents.is_some() {
                                return Err(de::Error::duplicate_field("contents"));
                            }

                            let text = value.get("text").and_then(|value| value.as_str());

                            if let Some(text) = text {
                                contents = match text.chars().next() {
                                    Some('/') => {
                                        let command = String::from(&text[1..]);
                                        let chat_id = value["chat"]["id"]
                                            .as_u64()
                                            .ok_or_else(|| {
                                                de::Error::custom("Can not parse chat id")
                                            })?;
                                        Some(Contents::Command(Command {
                                            command,
                                            chat_id,
                                        }))
                                    },
                                    _ => Some(Contents::Message(
                                        Message::deserialize(value)
                                            .map_err(de::Error::custom)?,
                                    )),
                                }
                            }
                        },
                        _ => {},
                    }
                }

                let update_id =
                    update_id.ok_or_else(|| de::Error::missing_field("update_id"))?;
                let contents = contents.unwrap_or(Contents::None);

                Ok(Update {
                    update_id,
                    contents,
                })
            }
        }

        const FIELDS: &[&str] = &["update_id", "contents"];
        deserializer.deserialize_struct("Update", FIELDS, UpdateVisitor)
    }
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub message_id: u64,
    pub text: String,
    pub from: User,
    pub chat: Chat,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct Chat {
    pub id: u64,
}

#[derive(Debug)]
pub struct Command {
    pub command: String,
    pub chat_id: u64,
}

#[derive(Debug)]
pub enum Contents {
    Command(Command),
    Message(Message),
    None,
}


#[derive(Deserialize)]
pub struct CallbackMessage;

#[derive(Serialize)]
#[serde(rename = "message")]
pub struct ResponseMessage {
    pub chat_id: u64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<String>,
}


impl TryFrom<ResponseMessage> for Body {
    type Error = failure::Error;

    fn try_from(value: ResponseMessage) -> Result<Self, Self::Error> {
        let ser = serde_json::to_vec(&value)?;
        Ok(Body::from(ser))
    }
}
