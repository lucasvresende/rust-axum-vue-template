# Axum + Vue full-stack template

A Rust and Vue starting point inspired by [FastAPI’s full-stack template](https://github.com/fastapi/full-stack-fastapi-template). It combines an authenticated JSON API with an admin workspace for managing personal inventory and viewing users.

## Contents

- [What is included](#what-is-included)
- [Prerequisites](#prerequisites)
- [Local development](#local-development)
- [Feature guide](#feature-guide)
- [Configuration](#configuration)
- [API and authentication](#api-and-authentication)
- [Project structure](#project-structure)
- [Frontend development](#frontend-development)
- [Database migrations](#database-migrations)
- [Compile-time SQL verification](#compile-time-sql-verification)
- [Formatting and testing](#formatting-and-testing)
- [Production hosting](#production-hosting)
- [Troubleshooting](#troubleshooting)
- [References](#references)

## What is included

- **Axum 0.8** API with typed JSON, JWT authentication, CORS, request tracing, and Swagger UI.
- **SQLx 0.9 and PostgreSQL 18**, compile-time checked queries, and embedded migrations.
- **Argon2id** password hashing, bootstrap administrator, admin invitations, profile and email changes, password recovery, and an admin-only user directory.
- Items with owner-scoped create, read, update, and delete operations, quantities, and creation timestamps.
- **Vue 3.5, Vite, PrimeVue 4.5 (Aura), and Tailwind CSS 4**, with a Sakai-inspired responsive workspace, dark mode, and toast feedback.
- Rust API tests and Playwright browser tests; Docker Compose services for PostgreSQL and Mailpit.

Mailpit receives development email. The API queues encrypted invitation, recovery, and account-change mail and retries SMTP delivery.

## Prerequisites

- Rust and Cargo supporting edition 2024; the crate declares Rust 1.88 as its minimum version.
- Node.js compatible with the locked Vite version (Node 22.12+ on the 22.x line is a suitable baseline) and npm.
- Docker Engine with the Compose plugin, or an independently provisioned PostgreSQL server.
- The SQLx CLI matching the backend’s 0.9 release.

Commands below use Bash and start from the repository root unless stated otherwise.

## Local development

### 1. Configure and start the database

```bash
cp .env.example .env
docker compose up -d db mailpit
docker compose ps
```

Copy the environment file only on first setup so existing configuration is preserved. Wait for the database to report healthy. Compose exposes PostgreSQL on host port **55432**, avoiding the usual local PostgreSQL port.

Export the configuration into the shell that will run Cargo:

```bash
set -a
source .env
set +a
```

The API reads process environment variables; it does not load `.env` itself. SQLx tooling can read `.env`, but that does not configure the running API process.

### 2. Apply migrations and run the API

Install the CLI once, then migrate before the first compilation:

```bash
cargo install sqlx-cli --version 0.9.0 --locked --no-default-features --features rustls,postgres
cargo sqlx migrate run
cargo run
```

SQLx checks queries against the database during compilation. Startup also applies embedded migrations and creates the configured administrator if its email does not already exist.

### 3. Start the frontend

In a second terminal, from the repository root:

```bash
cd frontend
npm ci
npm run dev
```

Open [the workspace](http://localhost:5173). On a new database, the example bootstrap credentials are `admin@example.com` / `development-admin-password`. Enter them in the empty login form. Existing administrators keep their previously configured passwords.

| Service    | Local address                       | Purpose                           |
| ---------- | ----------------------------------- | --------------------------------- |
| Workspace  | http://localhost:5173               | Vue development server            |
| API health | http://localhost:8000/api/v1/health | HTTP liveness response            |
| Swagger UI | http://localhost:8000/docs/         | Interactive API documentation     |
| Mailpit    | http://localhost:8025               | Development email inbox           |
| PostgreSQL | localhost:55432                     | Database connection from the host |
| SMTP       | localhost:1025                      | Mailpit SMTP listener             |

Stop the API and Vite with Ctrl+C. Use `docker compose stop` to stop the development services while keeping their containers.

## Feature guide

The initial administrator is created at API startup from `FIRST_SUPERUSER_EMAIL` and
`FIRST_SUPERUSER_PASSWORD`. That account is the only way to administer users; there
is no public sign-up or role-promotion screen or endpoint. An administrator creates
regular users, and each new user chooses their own password through an emailed link.
An inactive account cannot sign in or use an existing bearer token.

| Feature | How to use it | Where it is implemented |
| ------- | ------------- | ----------------------- |
| Sign in and sign out | Enter your email and password at `/login`; use **Sign out** in the header. Bookmarked workspace pages resume after sign-in. | [`src/auth.rs`](src/auth.rs), [`frontend/src/components/LoginForm.vue`](frontend/src/components/LoginForm.vue), [`frontend/src/App.vue`](frontend/src/App.vue), [`frontend/src/router.ts`](frontend/src/router.ts) |
| Overview | Open **Overview** for your item count, role, session status, and a preview of your items. Admins also see the user directory. | [`frontend/src/components/DashboardHero.vue`](frontend/src/components/DashboardHero.vue), [`frontend/src/components/DashboardStats.vue`](frontend/src/components/DashboardStats.vue), [`frontend/src/App.vue`](frontend/src/App.vue) |
| Personal inventory | Open **My items** to create, edit, or delete an item. Each item has a title, description, non-negative quantity, and creation time; only its owner can view or change it. | [`src/routes/items.rs`](src/routes/items.rs), [`frontend/src/components/ItemsTable.vue`](frontend/src/components/ItemsTable.vue), [`frontend/src/components/CreateItemDialog.vue`](frontend/src/components/CreateItemDialog.vue) |
| Table controls | Search, sort, filter, select visible columns, and paginate the items and users tables. | [`frontend/src/components/WorkspaceTable.vue`](frontend/src/components/WorkspaceTable.vue), [`frontend/src/components/ColumnFilter.vue`](frontend/src/components/ColumnFilter.vue) |
| User directory | As an admin, open **Users** to inspect names, emails, roles, invitation state, and active status. Members cannot open this page or call the directory endpoint. | [`src/routes/users.rs`](src/routes/users.rs), [`frontend/src/components/UsersTable.vue`](frontend/src/components/UsersTable.vue) |
| Invite a user | On **Users**, choose **New user**, enter a name and email, then **Send invitation**. The form has no password field; the account is a regular user awaiting setup. | [`src/routes/users.rs`](src/routes/users.rs), [`frontend/src/components/UsersTable.vue`](frontend/src/components/UsersTable.vue), [`migrations/20260928120000_user_management.sql`](migrations/20260928120000_user_management.sql) |
| Resend an invitation | Use **Resend invite** for an active user still awaiting password setup. The previous link stops working. | [`src/routes/users.rs`](src/routes/users.rs), [`frontend/src/components/UsersTable.vue`](frontend/src/components/UsersTable.vue) |
| Activate or deactivate | Use the row action on **Users**. Deactivation immediately blocks login and existing tokens. Reactivation permits a fresh login; it does not revive an old token. Admins cannot deactivate themselves or the last active admin. | [`src/routes/users.rs`](src/routes/users.rs), [`src/auth.rs`](src/auth.rs), [`frontend/src/components/UsersTable.vue`](frontend/src/components/UsersTable.vue) |
| First password setup | Open the link in the invitation email, enter and confirm a password at `/set-password`, then sign in normally. | [`src/routes/users.rs`](src/routes/users.rs), [`frontend/src/components/AccountFlows.vue`](frontend/src/components/AccountFlows.vue) |
| Password recovery | Choose **Forgot password?** at `/login`, enter your email, open the recovery link, and set a new password at `/reset-password`. The request page always gives a generic response. | [`src/routes/users.rs`](src/routes/users.rs), [`frontend/src/components/AccountFlows.vue`](frontend/src/components/AccountFlows.vue) |
| Edit your profile | Open **Profile** to edit your display name. It saves immediately and queues a confirmation notice to your current email. | [`src/routes/users.rs`](src/routes/users.rs), [`frontend/src/components/ProfilePage.vue`](frontend/src/components/ProfilePage.vue) |
| Change your login email | In **Profile**, enter the new email and your current password. Confirm the link sent to the new address; until then, the old address remains your login. The old address receives a change notice. | [`src/routes/users.rs`](src/routes/users.rs), [`frontend/src/components/ProfilePage.vue`](frontend/src/components/ProfilePage.vue), [`frontend/src/components/AccountFlows.vue`](frontend/src/components/AccountFlows.vue) |
| Appearance and navigation | Toggle dark mode or collapse the workspace navigation from the header. Workspace URLs support bookmarks, refresh, and browser history. | [`frontend/src/components/AppHeader.vue`](frontend/src/components/AppHeader.vue), [`frontend/src/components/WorkspaceNav.vue`](frontend/src/components/WorkspaceNav.vue), [`frontend/src/router.ts`](frontend/src/router.ts) |
| API documentation and health | Use `/docs/` for Swagger UI, `/api-docs/openapi.json` for the schema, and `/api/v1/health` for liveness. | [`src/routes/docs.rs`](src/routes/docs.rs), [`src/routes/health.rs`](src/routes/health.rs) |
| Outbound email | Locally, inspect invitation, recovery, and change notices at [Mailpit](http://localhost:8025). The API encrypts queued bodies and retries SMTP failures. | [`src/mail.rs`](src/mail.rs), [`docker-compose.yml`](docker-compose.yml), [`src/state.rs`](src/state.rs) |

### Account and email behavior

- A new user has no usable password until they redeem an invitation. Setup links
  expire after **24 hours**; recovery and email-confirmation links expire after
  **30 minutes**. Each link is single use. Request a new invitation from an admin
  if setup expires; use **Forgot password?** if an established password is lost.
- New passwords must be **15 to 1024 characters**. Existing passwords remain valid
  until changed. Setup and recovery do not sign the user in automatically.
- Password setup, recovery, email confirmation, and account status changes revoke
  previously issued bearer tokens. A completed email change also requires signing
  in with the new address.
- Recovery requests are limited to three per email per hour, and redemption
  attempts to ten per token per hour. Unknown, inactive, and rate-limited recovery
  requests receive the same public message. Mail delivery is asynchronous; use
  Mailpit in development to check that it arrived.
- The admin cannot see or set a user's password. Admin creation accepts a name
  and email only; password hashes and raw link tokens are not returned by the API.
- PostgreSQL stores a hash of each single-use link token in `action_tokens`.
  Email bodies containing links are encrypted in `mail_outbox`; the SMTP worker
  retries failures and removes each queued message after successful delivery.

## Configuration

| Variable                   | Example/default                                            | Meaning                                                         |
| -------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------- |
| `DATABASE_URL`             | `postgres://app:app@localhost:55432/app` in `.env.example` | Required database URL; used at build time and runtime.          |
| `APP_ADDR`                 | `0.0.0.0:8000`                                             | API bind address and port.                                      |
| `JWT_SECRET`               | Development fallback if unset                              | Signing secret; configure a strong unique value for deployment. |
| `FRONTEND_ORIGIN`          | `http://localhost:5173`                                    | Single browser origin allowed by CORS.                          |
| `FIRST_SUPERUSER_EMAIL`    | `admin@example.com`                                        | Email used when creating the initial administrator.             |
| `FIRST_SUPERUSER_PASSWORD` | `development-admin-password` | Required when that account is first created; 15 to 1024 characters. |
| `SMTP_HOST`, `SMTP_PORT` | `localhost`, `1025` in `.env.example` | Outbound SMTP endpoint. |
| `SMTP_TLS` | `none` locally; `starttls` or `tls` in deployment | SMTP transport security. |
| `SMTP_USERNAME`, `SMTP_PASSWORD` | unset locally | Optional SMTP credentials. |
| `MAIL_FROM` | `Workspace <noreply@example.com>` | Sender mailbox. |
| `MAIL_ENCRYPTION_KEY` | 64 hex digits in `.env.example` | 32-byte AES key for queued email; use a unique secret in deployment and retain it while mail is queued. |
| `PUBLIC_URL` | `http://localhost:5173` | Trusted base URL for email links; use HTTPS in deployment. |

Existing installations must add the new mail settings to `.env` before restarting. Changing bootstrap credentials does not update an existing account. Email addresses are normalized to lowercase. Use `localhost:1025` when the API runs on the host and `mailpit:1025` when it runs inside the Compose network. Do not use the example encryption key or bootstrap password in deployment.

To use another API port:

```bash
APP_ADDR=0.0.0.0:8001 cargo run
```

Also update the `/api` proxy target in `frontend/vite.config.ts` and restart Vite. If changing the frontend origin, update `FRONTEND_ORIGIN` and restart the API.

## API and authentication

All application endpoints use the `/api/v1` prefix.

| Method | Path | Access and behavior |
| ------ | ---- | ------------------- |
| GET | `/health` | Public liveness check. |
| POST | `/login` | Public email/password login. |
| GET / POST | `/users` | Active admin only; list users or create a regular user and queue an invitation. |
| PUT | `/users/{id}/status` | Admin only; activate or deactivate a user. |
| POST | `/users/{id}/resend-invitation` | Admin only; issue a fresh link for a user without a password. |
| GET / PUT | `/users/me` | Active user; read profile or edit display name. |
| POST | `/users/me/email-change` | Active user; request a verified login-email change with current password. |
| POST | `/users/confirm-email` | Public; redeem an email-confirmation link. |
| POST | `/password/setup` | Public; redeem an invitation link to choose a password. |
| POST | `/password/recovery-requests` | Public; request a recovery link with a generic response. |
| POST | `/password/recover` | Public; redeem a recovery link to set a new password. |
| GET / POST | `/items` | Active user; list or create owned items. |
| PUT / DELETE | `/items/{id}` | Active owner; update or delete an item. |

JWTs expire after 24 hours. Every authenticated request reloads the active user and authentication version from PostgreSQL, so deactivation, password resets, email changes, and permission changes take effect immediately. Even superusers see and modify only their own items. Missing items and items owned by another user both return 404.

The frontend stores the bearer token in local storage and removes it on logout. Send it as `Authorization: Bearer <access_token>` when calling authenticated endpoints directly. API errors normally use `{ "detail": "..." }`; Axum request-extraction errors can be plain text. Successful account-link actions and item deletion return 204 with no response body.

### Request bodies for account operations

These JSON bodies are accepted under `/api/v1`; admin and profile calls also require
the bearer token returned by `/login`. The token in an emailed URL is sent in the
JSON body when redeeming that link.

| Method and path | JSON body | Result |
| --------------- | --------- | ------ |
| `POST /login` | `{"email":"admin@example.com","password":"…"}` | `access_token` and `token_type`. |
| `POST /users` | `{"email":"person@example.com","full_name":"Person"}` | New regular user and queued invitation; no password input or output. |
| `PUT /users/{id}/status` | `{"is_active":false}` or `{"is_active":true}` | Updated user. |
| `POST /users/{id}/resend-invitation` | No body | 204; available only while the active user has no password. |
| `PUT /users/me` | `{"full_name":"New Name"}` | Updated user and queued email notice. |
| `POST /users/me/email-change` | `{"email":"new@example.com","current_password":"…"}` | 204; the login email remains unchanged until confirmation. |
| `POST /users/confirm-email` | `{"token":"…"}` | 204; login with the new address. |
| `POST /password/recovery-requests` | `{"email":"person@example.com"}` | Generic message for every account state. |
| `POST /password/setup` or `/password/recover` | `{"token":"…","password":"…"}` | 204; sign in normally afterward. |

The item endpoints accept `{"title":"…","description":"…","quantity":1}` for
creation and updates. `description` defaults to empty and `quantity` to 1 on
creation; updates require all three fields. IDs, ownership, and creation times are
assigned by the API.

### Interactive documentation

With the API running on its default port, open [Swagger UI](http://localhost:8000/docs/).
The [OpenAPI JSON specification](http://localhost:8000/api-docs/openapi.json) is also served by Axum.
Use the configured API port if `APP_ADDR` differs. These documentation routes are public.

To try authenticated requests:

1. Expand `POST /api/v1/login`, click **Try it out**, and submit your email and password as JSON.
2. Copy `access_token` from the response.
3. Click **Authorize**, paste the token without the `Bearer ` prefix, and confirm.
4. Execute the user and item endpoints. Listing all users requires a superuser account.

OpenAPI schemas are derived from `src/models.rs`; endpoint documentation lives beside the handlers.
`src/routes/docs.rs` collects the endpoints and defines JWT bearer authentication.
When adding a route, annotate its handler with `#[utoipa::path(...)]` and add it to the `ApiDoc` paths list.
Swagger UI assets are bundled through the `vendored` feature, so the running server does not depend on a CDN.

When serving behind a reverse proxy, forward `/docs`, `/docs/`, and `/api-docs/` to Axum
alongside `/api/`; otherwise the frontend SPA fallback will intercept documentation requests.

## Project structure

The backend is organized by responsibility:

- `src/main.rs`: logging, environment configuration, and HTTP server startup.
- `src/db.rs`: database connection, migrations, and initial superuser creation.
- `src/state.rs`: shared database pool, JWT secret, mail encryption key, and public URL.
- `src/mail.rs`: encrypted durable email queue and SMTP delivery worker.
- `src/routes/mod.rs`: route registration, CORS, and request tracing.
- `src/routes/docs.rs`: OpenAPI specification and Swagger UI.
- `src/routes/health.rs`, `users.rs`, and `items.rs`: health, user, and item handlers.
- `src/auth.rs`: password hashing, JWT creation and verification, and login.
- `src/models.rs`: request and response types.
- `src/error.rs`: shared API errors and HTTP error responses.

`frontend/src/App.vue` is the dashboard shell; `frontend/src/api.ts` centralizes authenticated API calls.

Other important files:

- `frontend/src/router.ts`: workspace routes and token-presence navigation guards.
- `frontend/src/components/`: reusable forms, tables, navigation, and dialogs.
- `frontend/src/types.ts`: frontend user and item types.
- `frontend/tests/` and `frontend/playwright.config.ts`: browser tests and development-server setup.
- `src/tests.rs`: in-process API and database integration tests.
- `migrations/`: versioned SQL schema changes; applied files must remain unchanged.
- `.env.example` and `docker-compose.yml`: local configuration and supporting services.

## Frontend development

The login, overview, inventory, users, and profile pages follow [Sakai's official layout](https://github.com/primefaces/sakai-vue/tree/master/src/layout) and [dashboard examples](https://github.com/primefaces/sakai-vue/tree/master/src/components/dashboard), adapted to the existing API data. Navigation can be collapsed from the topbar and stacks above the content on small screens.

Styling uses Tailwind CSS v4 via `@tailwindcss/vite` and the official `tailwindcss-primeui` integration. Add utility classes directly to Vue templates; semantic utilities such as `bg-primary`, `border-surface`, and `text-muted-color` follow the Aura theme. PrimeVue controls retain their Aura styling, with the `primevue` CSS layer before Tailwind utilities so utilities can override controls. See the [official PrimeVue Tailwind guide](https://primevue.org/tailwind/).

`frontend/src/style.css` contains the Tailwind imports and shared base styles. The `.app-dark` class is applied to the document root, keeping Tailwind, PrimeVue, and teleported dialogs in sync. No separate Tailwind config file is needed for v4.

Items include a non-negative integer `quantity` (defaults to 1 for existing items and requests that omit it) and a server-generated `created_at` timestamp. The items table can sort quantity and creation time, and formats creation dates in the browser's local time. Apply migrations before compiling changed SQLx queries: `cargo sqlx migrate run` with `DATABASE_URL` configured. Startup also runs pending migrations.

## Database migrations

Run these commands from the repository root. The project uses SQLx 0.9, PostgreSQL, and versioned SQL files in `migrations/`. See the [SQLx CLI documentation](https://github.com/transact-rs/sqlx/blob/v0.9.0/sqlx-cli/README.md).

### Install and connect

Install the matching CLI once:

```bash
cargo install sqlx-cli --version 0.9.0 --locked --no-default-features --features rustls,postgres
cargo sqlx --version
```

Create `.env` from `.env.example` if it does not exist, then start PostgreSQL and export the connection settings:

```bash
docker compose up -d db
set -a; source .env; set +a
```

`DATABASE_URL` must point to the intended database. The Compose service creates the configured `app` database on first startup; for a separately provisioned PostgreSQL server, `cargo sqlx database create` creates the database named in `DATABASE_URL` if your role has permission.

### Apply and inspect migrations

```bash
cargo sqlx migrate info
cargo sqlx migrate run
cargo sqlx migrate info
```

SQLx records applied versions and checksums in `_sqlx_migrations`. Run pending migrations **before compiling** code that references new tables or columns: `query!` and `query_as!` check the live schema during compilation.

The application also applies pending embedded migrations on startup. This happens after compilation, so it cannot prepare the database for a first build. `build.rs` watches `migrations/` and triggers recompilation when migration files change; rebuild the application to embed newly added migrations.

### Create a schema change

```bash
cargo sqlx migrate add --timestamp --simple add_item_location
```

Edit the generated `migrations/<timestamp>_add_item_location.sql`, for example:

```sql
ALTER TABLE items ADD COLUMN location TEXT NOT NULL DEFAULT '';
```

Then apply the migration and validate the corresponding Rust changes:

```bash
cargo sqlx migrate run
cargo check
cargo test
```

Commit the migration together with the application changes. New versions must sort after all previously applied versions. Do not edit, rename, or delete an applied migration; make corrections in a new migration so deployed databases retain a consistent history. If SQLx reports a checksum mismatch, restore the original applied file and add a follow-up migration.

### Reversible migrations

For a new change that needs an explicit rollback, generate an up/down pair instead:

```bash
cargo sqlx migrate add --timestamp --reversible add_item_location
```

Write the schema change in `<timestamp>_add_item_location.up.sql` and its inverse in `<timestamp>_add_item_location.down.sql`. These are alternatives to the simple migration example above. For that example, the down SQL would be:

```sql
ALTER TABLE items DROP COLUMN location;
```

On a development database, inspect the rollback before running it:

```bash
cargo sqlx migrate revert --dry-run
cargo sqlx migrate revert
cargo sqlx migrate info
```

A rollback can discard data (the example drops all stored locations). The repository's existing `.sql` migrations have no down files and cannot be reverted with this command; correct those with a new forward migration. Stop the app before reverting: restarting it applies pending embedded up migrations again. Reapply a reverted migration with `cargo sqlx migrate run` before compiling code that depends on it.

### Recovering a local development database

The initial migration uses `IF NOT EXISTS` to accommodate tables created manually during local bootstrap. Record the pending migrations before rebuilding:

```bash
set -a; source .env; set +a
cargo sqlx migrate run
cargo check
cargo run
```

This assumes the manually created tables match the initial schema; `IF NOT EXISTS` does not reconcile different columns or constraints. Later migrations are applied once through SQLx's migration history.

## Compile-time SQL verification

With PostgreSQL running and `DATABASE_URL` configured:

```bash
set -a; source .env; set +a
cargo sqlx migrate run
cargo check
cargo test
```

For builds without a database connection, generate query metadata against the migrated database and commit the resulting `.sqlx/` directory:

```bash
cargo sqlx prepare
SQLX_OFFLINE=true cargo check
```

Regenerate metadata whenever SQL queries or their schema change. A CI job with a migrated database can verify that the committed metadata is current:

```bash
cargo sqlx migrate run
cargo sqlx prepare --check
```

Offline metadata supports compilation only; the running API still needs PostgreSQL and applies its embedded migrations at startup.

## Formatting and testing

### Formatting

Use rustfmt for Rust and the pinned Prettier dependency for frontend code, templates, styles, and configuration:

```bash
cargo fmt --all
cd frontend
npm run format
```

Check formatting without modifying files:

```bash
cargo fmt --all -- --check
cd frontend
npm run format:check
```

The frontend uses two-space indentation, semicolons, and double-quoted JavaScript strings. Generated output, dependency files, and TypeScript build metadata are excluded from Prettier. Do not reformat applied SQL migrations: their checksums are recorded by SQLx.

### Backend tests

Run `cargo test` with the local PostgreSQL service available and `DATABASE_URL`
configured (SQLx also reads the root `.env`). The database used for compile-time
query checks must already have the migrations applied. The database role must
have permission to create databases: each `#[sqlx::test]` creates its own isolated
database, applies migrations, and removes the database after a successful test.
Failed test databases are retained by SQLx for debugging. Use a local development
PostgreSQL instance for this suite.

The backend suite covers invitations, recovery, token expiry and concurrent
redemption, throttling, password and email-change revocation, admin permissions,
SMTP retry queuing, item ownership and CRUD, quantity validation, profile
isolation, HTTP contracts, CORS, password hashing, migration constraints, and
Swagger/OpenAPI.

```bash
set -a
source .env
set +a
cargo sqlx migrate run
cargo check
cargo test
```

### Frontend build and browser tests

From the repository root:

```bash
cd frontend
npm ci
npm run build
npx playwright install chromium
npm run test:e2e
```

Playwright starts Vite on `127.0.0.1:5173`, or reuses an existing server there. Browser tests mock API responses, so they do not require a running API.

The browser suite covers the invitation, recovery, email-confirmation, admin, and
profile screens alongside inventory, navigation, tables, and mobile layout.
The dashboard test writes `artifacts/dashboard.png` at the repository root; failures can also produce screenshots under `frontend/test-results/`. On Linux systems missing browser libraries, use `npx playwright install --with-deps chromium` with the appropriate system permissions.

## Production hosting

Workspace pages use Vue Router history URLs: `/overview`, `/items`, `/users`, and `/profile`. Public account pages use `/login`, `/forgot-password`, `/set-password`, `/reset-password`, and `/confirm-email`.
Navigation supports browser Back/Forward, bookmarks, and refreshes. Opening a page
while signed out redirects to `/login?redirect=...` and continues to the requested
workspace page after sign-in. Only known workspace paths are accepted as redirect
destinations. Signing out goes to `/login`; signed-in visits to `/login` go to
`/overview`.
The Users page is restricted to administrators; members are returned to Overview. Email links use `PUBLIC_URL`, so configure it to the deployed HTTPS frontend origin.
The root URL and unknown frontend paths redirect to `/overview`.

Build the frontend with `cd frontend && npm run build`, then serve `frontend/dist`.
The production host must serve `index.html` for frontend page URLs, preserve static
asset requests, and forward `/api/` requests to Axum. Vite's development proxy is
not part of the production build.

For example, inside an Nginx `server` block (adjust paths and the API upstream for
your deployment):

```nginx
root /srv/app/frontend/dist;
index index.html;

location /api/ {
    proxy_pass http://127.0.0.1:8000;
}

location = /docs {
    proxy_pass http://127.0.0.1:8000;
}

location /docs/ {
    proxy_pass http://127.0.0.1:8000;
}

location /api-docs/ {
    proxy_pass http://127.0.0.1:8000;
}

location /assets/ {
    try_files $uri =404;
}

location / {
    try_files $uri $uri/ /index.html;
}
```

The fallback serves the app without redirecting the browser away from the requested
page. API errors and missing built assets are not rewritten to HTML. If using a
static hosting provider, configure its equivalent SPA fallback and API proxy.

Build the backend with `cargo build --release` against a migrated database, or prepare offline query metadata first. Run `target/release/axum-vue-template` with the required environment variables and database access. Axum serves the API and documentation; the frontend assets are served separately by your web host.

Use deployment-specific database credentials, a strong `JWT_SECRET`, a unique `MAIL_ENCRYPTION_KEY`, an SMTP sender with TLS, and a changed bootstrap password. Terminate HTTPS at your proxy; configure `FRONTEND_ORIGIN` and `PUBLIC_URL` for the deployed frontend. Keep the mail encryption key while queued mail remains, because the worker needs it to retry delivery. The supplied Compose file is for development; plan persistent database storage and backups separately.

## Troubleshooting

| Symptom                                                   | What to check                                                                                          |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| SQLx compilation fails with missing tables or columns     | Export `DATABASE_URL` and apply migrations before building.                                            |
| Database connection refused                               | Check `docker compose ps` and use host port `55432` for the supplied database.                         |
| API reports a missing environment variable                | Source `.env` with `set -a` in the same terminal before `cargo run`.                                   |
| Frontend requests fail after changing API ports           | Update the Vite proxy target and restart Vite.                                                         |
| Login fails after editing bootstrap credentials           | Existing accounts retain their original credentials; the environment variables only seed new accounts. |
| Invitation or recovery email does not arrive             | Check Mailpit or the configured SMTP service, `MAIL_FROM`, `SMTP_TLS`, and the queued `mail_outbox` rows; failed deliveries are retried. |
| Email link opens the wrong host or expires                | Set `PUBLIC_URL` to the deployed HTTPS frontend origin and request a fresh link after expiry. |
| Refreshing `/items` or `/users` returns 404 in production | Configure the SPA fallback described above.                                                            |
| Swagger UI returns the frontend HTML                      | Forward `/docs`, `/docs/`, and `/api-docs/` to Axum.                                                   |
| Backend tests cannot create a database                    | Use a development PostgreSQL role with database-creation permission.                                   |
| Browser tests cannot launch Chromium                      | Install Playwright’s browser and required system libraries.                                            |

## References

- [Upstream FastAPI template](https://github.com/fastapi/full-stack-fastapi-template)
- [Axum documentation](https://docs.rs/axum/latest/axum/)
- [SQLx checked macros and migrations](https://docs.rs/sqlx/latest/sqlx/)
- [PrimeVue documentation](https://primevue.org/)

## License

This project is licensed under the [MIT License](LICENSE).
