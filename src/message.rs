use chrono::NaiveDateTime;
use super::utils::timestamp_ser;

#[derive(Debug,Serialize)]
pub struct Message {
    pub id: i32,
    pub author: String,
    #[serde(serialize_with = "timestamp_ser")]
    pub timestamp: NaiveDateTime,
    pub content: String,
    /// tags on 32bits:
    /// tags & 1 = logged_in
    /// tags & 2 >> 1 = generated // is this message generated from another message
    /// tags & 4 >> 2 = bot // is this message sent by a bot ?
    /// tags & 8 >> 3 = no_notif // should this message ignore notifications rules and not notify him
    /// tags & (2^4 + 2^5 + 2^6 + 2^7) = show_value
    /// everything else: reserved for later use
    ///
    /// show_value : u4; 
    /// show_value = 0: no change;
    /// show_value = 1 to 4; 'hidden' message, with 4 being more hidden than 1
    /// show_value = 9 to 12; 'important' message, with 12 being more important
    ///
    /// ^ these are basically ignored by the server and are only implementation dependant
    /// you can set up your client to never show show_value = 4, and show show_value = 1 but not
    /// notify, ...
    pub tags: i32,
    pub color: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug,Deserialize)]
pub struct NewMessage {
    pub username: String,
    pub content: String,
    pub tags: Option<i32>,
    pub color: Option<String>,
    pub channel: Option<String>,
    pub auth_key: Option<String>,
}
