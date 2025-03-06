use serde::{
    de,
    de::{MapAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
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
            Contents::CallbackMessage(callback_message) => Some(callback_message.message.chat.id),
            Contents::Current(chat_id) => Some(*chat_id),
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

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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
                        }
                        "callback_query" => {
                            if contents.is_some() {
                                return Err(de::Error::duplicate_field("contents"));
                            }

                            contents = Some(Contents::CallbackMessage(
                                CallbackMessage::deserialize(value).map_err(de::Error::custom)?,
                            ));
                        }
                        "message" => {
                            if contents.is_some() {
                                return Err(de::Error::duplicate_field("contents"));
                            }

                            let text = value.get("text").and_then(|value| value.as_str());

                            if let Some(text) = text {
                                contents = match text.chars().next() {
                                    Some('/') => {
                                        let command = String::from(&text[1..]);
                                        let chat_id =
                                            value["chat"]["id"].as_u64().ok_or_else(|| {
                                                de::Error::custom("Can not parse chat id")
                                            })?;
                                        if command == "current" {
                                            Some(Contents::Current(chat_id))
                                        } else {
                                            Some(Contents::Command(Command { command, chat_id }))
                                        }
                                    }
                                    _ => Some(Contents::Message(
                                        Message::deserialize(value).map_err(de::Error::custom)?,
                                    )),
                                }
                            }
                        }
                        "edited_message" => {
                            if contents.is_some() {
                                return Err(de::Error::duplicate_field("contents"));
                            }

                            contents = Some(Contents::Message(
                                Message::deserialize(value).map_err(de::Error::custom)?,
                            ));
                        }
                        _ => {}
                    }
                }

                let update_id = update_id.ok_or_else(|| de::Error::missing_field("update_id"))?;
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

#[derive(Deserialize, Debug)]
pub struct CallbackMessage {
    pub id: String,
    pub from: User,
    pub message: Message,
    pub data: CallbackData,
}

#[derive(Debug)]
pub struct CallbackData {
    pub command: String,
    pub message_id: Option<u64>,
}

impl Serialize for CallbackData {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        if let Some(message_id) = self.message_id {
            serializer.serialize_str(&format!("{}/{}", self.command, message_id))
        } else {
            serializer.serialize_str(&self.command)
        }
    }
}

impl<'de> Deserialize<'de> for CallbackData {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct CallbackDataVisitor;

        impl<'de> Visitor<'de> for CallbackDataVisitor {
            type Value = CallbackData;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct CallbackData")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                let mut iter = value.split('/');
                let command = iter
                    .next()
                    .ok_or_else(|| de::Error::custom("command not found"))?
                    .to_owned();
                let message_id = iter
                    .next()
                    .map(|message_id| message_id.parse())
                    .transpose()
                    .map_err(de::Error::custom)?;

                Ok(CallbackData {
                    command,
                    message_id,
                })
            }
        }

        deserializer.deserialize_any(CallbackDataVisitor)
    }
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
    CallbackMessage(CallbackMessage),
    Current(u64),
    None,
}

#[derive(Serialize)]
#[serde(rename = "message")]
pub struct ResponseMessage {
    pub chat_id: u64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<InlineKeyboardMarkup>,
}

#[derive(Serialize)]
pub struct InlineKeyboardMarkup {
    pub inline_keyboard: [Vec<InlineKeyboardButton>; 1],
}

#[derive(Serialize)]
pub struct InlineKeyboardButton {
    pub text: String,
    pub callback_data: CallbackData,
}
