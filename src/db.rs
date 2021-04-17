use lazy_static::lazy_static;

lazy_static!{
    pub static ref DB: sled::Db = sled::open("db").unwrap();
    pub static ref DB_SUBSCRIPTIONS: sled::Tree = DB.open_tree("subscriptions").unwrap();
}