use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, OsRng, rand_core::RngCore},
};
use chrono::{Duration, Utc};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::{env, sync::Arc, time::Duration as StdDuration};
use uuid::Uuid;

use crate::{
    error::{ApiError, Result},
    state::AppState,
};

pub(crate) async fn queue(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    recipient: &str,
    subject: &str,
    body: &str,
) -> Result<()> {
    let cipher = Aes256Gcm::new_from_slice(state.mail_key.as_ref()).expect("valid mail key");
    let mut nonce = [0_u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), body.as_bytes())
        .map_err(|_| ApiError::Bad("mail encryption failed"))?;
    sqlx::query(
        "INSERT INTO mail_outbox (id,recipient,subject,ciphertext,nonce) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(recipient)
    .bind(subject)
    .bind(ciphertext)
    .bind(nonce.to_vec())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) fn key_from_env() -> std::result::Result<[u8; 32], Box<dyn std::error::Error>> {
    let source = env::var("MAIL_ENCRYPTION_KEY")?;
    let bytes = hex::decode(source)?;
    Ok(bytes
        .try_into()
        .map_err(|_| "MAIL_ENCRYPTION_KEY must be 32 bytes in hex")?)
}

fn transport() -> std::result::Result<AsyncSmtpTransport<Tokio1Executor>, Box<dyn std::error::Error>>
{
    let host = env::var("SMTP_HOST")?;
    let port = env::var("SMTP_PORT")
        .unwrap_or_else(|_| "587".into())
        .parse()?;
    let mode = env::var("SMTP_TLS").unwrap_or_else(|_| "starttls".into());
    let mut builder = match mode.as_str() {
        "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host),
        "starttls" => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)?,
        "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&host)?,
        _ => return Err("SMTP_TLS must be none, starttls, or tls".into()),
    }
    .port(port)
    .timeout(Some(StdDuration::from_secs(10)));
    if let Ok(username) = env::var("SMTP_USERNAME") {
        builder = builder.credentials(Credentials::new(username, env::var("SMTP_PASSWORD")?));
    }
    Ok(builder.build())
}

pub(crate) fn start_worker(
    pool: PgPool,
    key: Arc<[u8; 32]>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sender = transport()?;
    let from = env::var("MAIL_FROM")?.parse::<lettre::message::Mailbox>()?;
    tokio::spawn(async move {
        let mut ticks = 0_u64;
        loop {
            if let Err(error) = deliver_one(&pool, key.as_ref(), &sender, &from).await {
                tracing::error!(%error, "mail delivery attempt failed");
            }
            ticks += 1;
            if ticks % 12 == 0 {
                if let Err(error) =
                    sqlx::query("DELETE FROM action_tokens WHERE expires_at < NOW()")
                        .execute(&pool)
                        .await
                {
                    tracing::error!(%error, "expired token cleanup failed");
                }
                if let Err(error) = sqlx::query(
                    "DELETE FROM request_limits WHERE window_started_at < NOW()-INTERVAL '2 hours'",
                )
                .execute(&pool)
                .await
                {
                    tracing::error!(%error, "rate-limit cleanup failed");
                }
            }
            tokio::time::sleep(StdDuration::from_secs(5)).await;
        }
    });
    Ok(())
}

async fn deliver_one(
    pool: &PgPool,
    key: &[u8; 32],
    sender: &AsyncSmtpTransport<Tokio1Executor>,
    from: &lettre::message::Mailbox,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query("SELECT id, recipient, subject, ciphertext, nonce, attempts FROM mail_outbox WHERE sent_at IS NULL AND next_attempt_at <= NOW() ORDER BY created_at FOR UPDATE SKIP LOCKED LIMIT 1")
        .fetch_optional(&mut *tx).await?;
    let Some(row) = row else {
        return Ok(());
    };
    let id: Uuid = row.get("id");
    let cipher = Aes256Gcm::new_from_slice(key)?;
    let ciphertext: Vec<u8> = row.get("ciphertext");
    let nonce: Vec<u8> = row.get("nonce");
    let body = String::from_utf8(
        cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| "mail decrypt failed")?,
    )?;
    let message = Message::builder()
        .from(from.clone())
        .to(row.get::<String, _>("recipient").parse()?)
        .subject(row.get::<String, _>("subject"))
        .body(body)?;
    match sender.send(message).await {
        Ok(_) => {
            sqlx::query("DELETE FROM mail_outbox WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        Err(error) => {
            let attempts: i32 = row.get("attempts");
            let delay = Duration::seconds(60 * (1_i64 << (attempts.min(6) as u32)));
            sqlx::query(
                "UPDATE mail_outbox SET attempts=attempts+1,next_attempt_at=$2 WHERE id=$1",
            )
            .bind(id)
            .bind(Utc::now() + delay)
            .execute(&mut *tx)
            .await?;
            tracing::warn!(%id, %error, "mail queued for retry");
        }
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    #[sqlx::test]
    async fn failed_smtp_delivery_remains_encrypted_and_is_retried(pool: PgPool) {
        let state = AppState {
            db: pool.clone(),
            secret: Arc::new("test-secret".into()),
            mail_key: Arc::new([7_u8; 32]),
            public_url: Arc::new("http://localhost:5173".into()),
        };
        let mut tx = pool.begin().await.unwrap();
        queue(
            &mut tx,
            &state,
            "recipient@example.com",
            "Test",
            "secret body",
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let before = sqlx::query("SELECT ciphertext FROM mail_outbox")
            .fetch_one(&pool)
            .await
            .unwrap();
        let ciphertext: Vec<u8> = before.get("ciphertext");
        assert!(!String::from_utf8_lossy(&ciphertext).contains("secret body"));
        let smtp = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
            .port(1)
            .timeout(Some(StdDuration::from_secs(1)))
            .build();
        let from = "Test <test@example.com>".parse().unwrap();
        deliver_one(&pool, state.mail_key.as_ref(), &smtp, &from)
            .await
            .unwrap();
        let row = sqlx::query("SELECT attempts, next_attempt_at FROM mail_outbox")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.get::<i32, _>("attempts"), 1);
        assert!(row.get::<chrono::DateTime<Utc>, _>("next_attempt_at") > Utc::now());
    }
}
