use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: PgPool,
    pub(crate) secret: Arc<String>,
    pub(crate) mail_key: Arc<[u8; 32]>,
    pub(crate) public_url: Arc<String>,
}
