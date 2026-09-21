use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub(crate) struct Health {
    pub(crate) status: &'static str,
}

#[derive(Serialize)]
pub(crate) struct Token {
    pub(crate) access_token: String,
    pub(crate) token_type: &'static str,
}

#[derive(Serialize, Clone)]
pub(crate) struct User {
    pub(crate) id: Uuid,
    pub(crate) email: String,
    pub(crate) full_name: String,
    pub(crate) is_active: bool,
    pub(crate) is_superuser: bool,
}

#[derive(Serialize)]
pub(crate) struct Item {
    pub(crate) id: Uuid,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) owner_id: Uuid,
    pub(crate) quantity: i32,
    pub(crate) created_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
pub(crate) struct Login {
    pub(crate) email: String,
    pub(crate) password: String,
}

#[derive(Deserialize)]
pub(crate) struct NewUser {
    pub(crate) email: String,
    pub(crate) password: String,
    pub(crate) full_name: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct NewItem {
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) quantity: Option<i32>,
}

#[derive(Deserialize)]
pub(crate) struct Profile {
    pub(crate) full_name: String,
}

#[derive(Deserialize)]
pub(crate) struct UpdateItem {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) quantity: i32,
}
