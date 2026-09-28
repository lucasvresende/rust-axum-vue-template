ALTER TABLE users ALTER COLUMN hashed_password DROP NOT NULL;
ALTER TABLE users ADD COLUMN auth_version BIGINT NOT NULL DEFAULT 0;
DROP TABLE password_reset_tokens;

CREATE TABLE action_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    purpose TEXT NOT NULL CHECK (purpose IN ('setup', 'recovery', 'email_change')),
    pending_email TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    CHECK ((purpose = 'email_change') = (pending_email IS NOT NULL))
);
CREATE INDEX action_tokens_user_purpose ON action_tokens(user_id, purpose);

CREATE TABLE mail_outbox (
    id UUID PRIMARY KEY,
    recipient TEXT NOT NULL,
    subject TEXT NOT NULL,
    ciphertext BYTEA NOT NULL,
    nonce BYTEA NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX mail_outbox_due ON mail_outbox(next_attempt_at) WHERE sent_at IS NULL;

CREATE TABLE request_limits (
    key_hash TEXT PRIMARY KEY,
    attempts INTEGER NOT NULL,
    window_started_at TIMESTAMPTZ NOT NULL
);
