use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Informant {
    pub id: i32,
    pub name: String,
    pub params: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl Informant {
    pub fn new(
        id: i32,
        name: String,
        params: serde_json::Value,
        created_at: chrono::NaiveDateTime,
        updated_at: chrono::NaiveDateTime,
    ) -> Self {
        Informant {
            id,
            name,
            params,
            created_at,
            updated_at,
        }
    }
}