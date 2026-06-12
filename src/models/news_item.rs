use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub id: i32,
    pub informant_id: i32,
    pub provided_id: String,
    pub index: String,
    pub title: String,
    pub content: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl NewsItem {
    pub fn new(
        id: i32,
        informant_id: i32,
        provided_id: String,
        index: String,
        title: String,
        content: String,
        created_at: chrono::NaiveDateTime,
        updated_at: chrono::NaiveDateTime,
    ) -> Self {
        NewsItem {
            id,
            informant_id,
            provided_id,
            index,
            title,
            content,
            created_at,
            updated_at,
        }
    }
}