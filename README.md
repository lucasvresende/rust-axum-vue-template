# Axum + Vue full-stack template

A Rust and Vue starting point inspired by [FastAPI’s full-stack template](https://github.com/fastapi/full-stack-fastapi-template). It combines an authenticated JSON API with an admin workspace for managing personal inventory and viewing users.

## Contents

- [What is included](#what-is-included)
- [Prerequisites](#prerequisites)
- [Local development](#local-development)
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
- **Argon2id** password hashing, bootstrap administrator, registration, profile updates, and an admin-only user directory.
- Items with owner-scoped create, read, update, and delete operations, quantities, and creation timestamps.
- **Vue 3.5, Vite, PrimeVue 4.5 (Aura), and Tailwind CSS 4**, with a Sakai-inspired responsive workspace, dark mode, and toast feedback.
- Rust API tests and Playwright browser tests; Docker Compose services for PostgreSQL and Mailpit.

Mailpit is supplied for future email development. The API does not currently send mail or implement password-reset endpoints.

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

Open [the workspace](http://localhost:5173). Development credentials are `admin@example.com` / `changethis`, unless changed in `.env`. The login form is prefilled with those defaults; enter your configured credentials if they differ.

| Service    | Local address                       | Purpose                           |
| ---------- | ----------------------------------- | --------------------------------- |
| Workspace  | http://localhost:5173               | Vue development server            |
| API health | http://localhost:8000/api/v1/health | HTTP liveness response            |
| Swagger UI | http://localhost:8000/docs/         | Interactive API documentation     |
| Mailpit    | http://localhost:8025               | Development email inbox           |
| PostgreSQL | localhost:55432                     | Database connection from the host |
| SMTP       | localhost:1025                      | Mailpit SMTP listener             |

Stop the API and Vite with Ctrl+C. Use `docker compose stop` to stop the development services while keeping their containers.

## Configuration

| Variable                   | Example/default                                            | Meaning                                                         |
| -------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------- |
| `DATABASE_URL`             | `postgres://app:app@localhost:55432/app` in `.env.example` | Required database URL; used at build time and runtime.          |
| `APP_ADDR`                 | `0.0.0.0:8000`                                             | API bind address and port.                                      |
| `JWT_SECRET`               | Development fallback if unset                              | Signing secret; configure a strong unique value for deployment. |
| `FRONTEND_ORIGIN`          | `http://localhost:5173`                                    | Single browser origin allowed by CORS.                          |
| `FIRST_SUPERUSER_EMAIL`    | `admin@example.com`                                        | Email used when creating the initial administrator.             |
| `FIRST_SUPERUSER_PASSWORD` | `changethis`                                               | Password used only when that account is first created.          |
| `SMTP_HOST`, `SMTP_PORT`   | `mailpit`, `1025` in `.env.example`                        | Reserved email settings; currently unused by the API.           |

Changing bootstrap credentials does not update an existing account. Use a lowercase administrator email, because login normalizes email addresses to lowercase. `mailpit` is a Compose network hostname; a future mail client running on the host would use `localhost:1025`.

To use another API port:

```bash
APP_ADDR=0.0.0.0:8001 cargo run
```

Also update the `/api` proxy target in `frontend/vite.config.ts` and restart Vite. If changing the frontend origin, update `FRONTEND_ORIGIN` and restart the API.

## API and authentication

All application endpoints use the `/api/v1` prefix.

| Method       | Path          | Access and behavior                                           |
| ------------ | ------------- | ------------------------------------------------------------- |
| GET          | `/health`     | Public liveness check; does not query PostgreSQL.             |
| POST         | `/login`      | Public; exchange email/password for a bearer token.           |
| POST         | `/users`      | Public registration; password must contain at least 12 bytes. |
| GET          | `/users`      | Active superusers only; list users.                           |
| GET / PUT    | `/users/me`   | Active user; read profile or update `full_name`.              |
| GET / POST   | `/items`      | Active user; list owned items or create an item.              |
| PUT / DELETE | `/items/{id}` | Active owner; update or delete an item.                       |

Tokens expire after 24 hours. Every authenticated request reloads the active user from PostgreSQL, so deactivation and permission changes take effect without waiting for token expiry. Even superusers see and modify only their own items. Missing items and items owned by another user both return 404.

The frontend stores the bearer token in local storage and removes it on logout. API errors normally use `{ "detail": "..." }`; Axum request-extraction errors can be plain text. A successful item deletion returns 204 with no response body.

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
- `src/state.rs`: shared database pool and JWT secret.
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

The login, overview, inventory, and users templates follow [Sakai's official layout](https://github.com/primefaces/sakai-vue/tree/master/src/layout) and [dashboard examples](https://github.com/primefaces/sakai-vue/tree/master/src/components/dashboard), adapted to the existing API data. Navigation can be collapsed from the topbar and stacks above the content on small screens.

Styling uses Tailwind CSS v4 via `@tailwindcss/vite` and the official `tailwindcss-primeui` integration. Add utility classes directly to Vue templates; semantic utilities such as `bg-primary`, `border-surface`, and `text-muted-color` follow the Aura theme. PrimeVue controls retain their Aura styling, with the `primevue` CSS layer before Tailwind utilities so utilities can override controls. See the [official PrimeVue Tailwind guide](https://primevue.org/tailwind/).

`frontend/src/style.css` contains the Tailwind imports and shared base styles. The `.app-dark` class is applied to the document root, keeping Tailwind, PrimeVue, and teleported dialogs in sync. No separate Tailwind config file is needed for v4.

Items include a non-negative integer `quantity` (defaults to 1 for existing items and requests that omit it) and a server-generated `created_at` timestamp. The admin table can sort both columns and formats creation dates in the browser's local time. Apply migrations before compiling changed SQLx queries: `cargo sqlx migrate run` with `DATABASE_URL` configured. Startup also runs pending migrations.

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

The backend suite covers registration/login, token validation and account
revocation, current admin permissions, item ownership and CRUD, quantity
validation, profile isolation, HTTP rejection statuses, CORS, safe error
responses, password hashing, and migration backfills and constraints. The existing
Swagger/OpenAPI test also runs as part of this command.

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

Playwright starts Vite on `127.0.0.1:5173`, or reuses an existing server there. Most tests mock API responses. The `dashboard.spec.ts` smoke test uses the real API and prefilled administrator credentials, so the API must be running with that development account for the full suite.

To run only the mocked browser tests without an API server:

```bash
npm run test:e2e -- --grep-invert "login dashboard is usable"
```

The smoke test writes `artifacts/dashboard.png` at the repository root; failures can also produce screenshots under `frontend/test-results/`. On Linux systems missing browser libraries, use `npx playwright install --with-deps chromium` with the appropriate system permissions.

## Production hosting

Workspace pages use Vue Router history URLs: `/overview`, `/items`, and `/users`.
Navigation supports browser Back/Forward, bookmarks, and refreshes. Opening a page
while signed out redirects to `/login?redirect=...` and continues to the requested
workspace page after sign-in. Only known workspace paths are accepted as redirect
destinations. Signing out goes to `/login`; signed-in visits to `/login` go to
`/overview`.
The Users page is restricted to administrators; members are returned to Overview.
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

Use deployment-specific database credentials, a strong `JWT_SECRET`, and a changed bootstrap password. Terminate HTTPS at your proxy and configure `FRONTEND_ORIGIN` for the deployed frontend. The supplied Compose file is for development; plan persistent database storage and backups separately.

## Troubleshooting

| Symptom                                                   | What to check                                                                                          |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| SQLx compilation fails with missing tables or columns     | Export `DATABASE_URL` and apply migrations before building.                                            |
| Database connection refused                               | Check `docker compose ps` and use host port `55432` for the supplied database.                         |
| API reports a missing environment variable                | Source `.env` with `set -a` in the same terminal before `cargo run`.                                   |
| Frontend requests fail after changing API ports           | Update the Vite proxy target and restart Vite.                                                         |
| Login fails after editing bootstrap credentials           | Existing accounts retain their original credentials; the environment variables only seed new accounts. |
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
