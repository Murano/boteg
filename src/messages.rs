use serde::{de, Deserialize, Serialize, Deserializer};
use serde_json::Value;
use serde::de::{Visitor, MapAccess};
use failure::_core::fmt::{Formatter, Debug};
use std::fmt;

#[derive(Debug)]
pub struct Update {
    pub update_id: u64,
    pub contents: Contents,
}

impl <'de>Deserialize<'de> for Update {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error> where
        D: Deserializer<'de> {

        struct UpdateVisitor;

        impl <'de>Visitor<'de> for UpdateVisitor {
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

                            let text = value.get("text").and_then(|value|value.as_str());

                            if let Some(text) = text {
                                contents = match text.chars().next() {
                                    Some('/') => Some(Contents::Command(Command(String::from(&text[1..])))),
                                    _ => Some(Contents::Message(Message::deserialize(value).map_err(de::Error::custom)?))
                                }
                            }
                        }
                        _ => {}
                    }
                }

                let update_id = update_id.ok_or_else(|| de::Error::missing_field("update_id"))?;
                let contents = contents.unwrap_or(Contents::None);

                Ok(Update{ update_id, contents })
            }
        }

        const FIELDS: &'static [&'static str] = &["update_id", "contents"];
        deserializer.deserialize_struct("Update", FIELDS, UpdateVisitor)
    }
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub text: String,
}

#[derive(Debug)]
pub struct Command(String);

#[derive(Debug)]
pub enum Contents {
    Command(Command),
    Message(Message),
    None
}


#[derive(Deserialize)]
pub struct CallbackMessage;

#[derive(Serialize)]
pub struct ResponseMessage;