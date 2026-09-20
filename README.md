# Axum + SQLx + Vue + PrimeVue template

A modern, deployable equivalent of FastAPI's full-stack template, adapted for Rust and Vue.

## Included

- **Axum 0.8** API with CORS, request tracing, typed JSON, error responses, health endpoint and JWT authentication.
- **SQLx 0.9** PostgreSQL access using `query!` / `query_as!` compile-time macros, embedded SQLx migrations, and a stable-Rust `build.rs` migration watcher.
- Secure **Argon2id** passwords, bootstrap superuser, self-service registration, current-user profile, admin users view, and ownership-safe items CRUD.
- **Vue 3.5**, Vite and **PrimeVue 4.5** (Aura preset), responsive dashboard, dark mode, toast feedback and keyboard-friendly components. This is the latest open-source PrimeVue line; PrimeVue 5 requires a PrimeUI license key and renders a license overlay without one.
- PostgreSQL 18 and Mailpit Docker development services; Playwright browser test and screenshot artifact.

## Run locally

```bash
cd /path/to/rust-axum-vue-template
cp .env.example .env
docker compose up -d db mailpit
set -a; source .env; set +a
cargo install sqlx-cli --version 0.9.0 --locked --no-default-features --features rustls,postgres
cargo sqlx migrate run
cargo run
```

If port 8000 is already used, run on another port (and set the matching Vite proxy target):

```bash
APP_ADDR=0.0.0.0:8001 cargo run
```

In a second terminal:

```bash
cd frontend
npm install
npm run dev
```

Open `http://localhost:5173`. The initial login is `admin@example.com` / `changethis`; replace this configuration before deployment. Mailpit is available at `http://localhost:8025`.

## Migration workflow

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

## Structure

`src/main.rs` contains the intentionally compact reference implementation. Split routes, authentication, models and services into modules as the application grows. `frontend/src/App.vue` is the dashboard shell; `frontend/src/api.ts` centralizes authenticated API calls.

## Verification

```bash
set -a; source .env; set +a
cargo sqlx migrate run
cargo check
cd frontend && npm run build && npx playwright test
```

## References

- [Upstream FastAPI template](https://github.com/fastapi/full-stack-fastapi-template)
- [Axum documentation](https://docs.rs/axum/latest/axum/)
- [SQLx checked macros and migrations](https://docs.rs/sqlx/latest/sqlx/)
- [PrimeVue documentation](https://primevue.org/)

## Admin UI and Tailwind

The login, overview, inventory, and users templates follow [Sakai's official layout](https://github.com/primefaces/sakai-vue/tree/master/src/layout) and [dashboard examples](https://github.com/primefaces/sakai-vue/tree/master/src/components/dashboard), adapted to the existing API data. Navigation can be collapsed from the topbar and stacks above the content on small screens.

Styling uses Tailwind CSS v4 via `@tailwindcss/vite` and the official `tailwindcss-primeui` integration. Add utility classes directly to Vue templates; semantic utilities such as `bg-primary`, `border-surface`, and `text-muted-color` follow the Aura theme. PrimeVue controls retain their Aura styling, with the `primevue` CSS layer before Tailwind utilities so utilities can override controls. See the [official PrimeVue Tailwind guide](https://primevue.org/tailwind/).

`frontend/src/style.css` contains the Tailwind imports and shared base styles. The `.app-dark` class is applied to the document root, keeping Tailwind, PrimeVue, and teleported dialogs in sync. No separate Tailwind config file is needed for v4.

Items include a non-negative integer `quantity` (defaults to 1 for existing items and requests that omit it) and a server-generated `created_at` timestamp. The admin table can sort both columns and formats creation dates in the browser's local time. Apply migrations before compiling changed SQLx queries: `cargo sqlx migrate run` with `DATABASE_URL` configured. Startup also runs pending migrations.

## Page URLs and production hosting

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

## License

This project is licensed under the [MIT License](LICENSE).
