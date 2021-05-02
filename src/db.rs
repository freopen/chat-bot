use lazy_static::lazy_static;
use rustbreak::{deser::Yaml, PathDatabase};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Subscription {
    RSS { url: String, last_entry: String },
}

impl PartialEq for Subscription {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Subscription::RSS { url, .. }, Subscription::RSS { url:other_url, .. }) => {
                url == other_url
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct SubscribeInfo {
    pub chat_to_sub: HashMap<i64, Vec<Subscription>>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct DBType {
    pub subscribe: SubscribeInfo,
}

lazy_static! {
    pub static ref DB: PathDatabase<DBType, Yaml> =
        PathDatabase::load_from_path_or_default("db/db".into()).unwrap();
}
