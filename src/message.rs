use chrono::NaiveDateTime;

#[derive(Debug,Serialize)]
pub struct Message {
    pub id: i32,
    pub author: String,
    pub timestamp: NaiveDateTime,
    pub content: String,
    pub tags: i32,
    pub color: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug,Deserialize)]
pub struct NewMessage {
    pub author: String,
    pub timestamp: NaiveDateTime,
    pub content: String,
    pub tags: i32,
    pub color: Option<String>,
    pub channel: Option<String>,
    pub auth_key: Option<String>,
}
