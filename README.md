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

## Compile-time SQL verification

SQLx macros intentionally require a database schema during `cargo check` / `cargo test`. Keep the database service running and export `DATABASE_URL` before compiling:

```bash
set -a; source .env; set +a
cargo check
cargo test
```

For hermetic CI, generate and commit SQLx offline metadata after migrations change:

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
cargo sqlx prepare
SQLX_OFFLINE=true cargo check
```

## Migration workflow

Create a timestamped SQL file under `migrations/`; the application runs its embedded migrations on startup. The build script ensures that a changed migration recompiles the embedded migrator on stable Rust. Do not edit a migration already deployed to an environment.

### Recovering a local development database

If an earlier attempt created tables manually and startup reports `relation "users" already exists`, rebuild and run once from the template directory. The initial migration is idempotent specifically for this local-bootstrap case and SQLx will record it on that run:

```bash
cd /path/to/rust-axum-vue-template
set -a; source .env; set +a
cargo run
```

## Structure

`src/main.rs` contains the intentionally compact reference implementation. Split routes, authentication, models and services into modules as the application grows. `frontend/src/App.vue` is the dashboard shell; `frontend/src/api.ts` centralizes authenticated API calls.

## Verification

```bash
set -a; source .env; set +a
cargo check
cd frontend && npm run build && npx playwright test
```

## References

- [Upstream FastAPI template](https://github.com/fastapi/full-stack-fastapi-template)
- [Axum documentation](https://docs.rs/axum/latest/axum/)
- [SQLx checked macros and migrations](https://docs.rs/sqlx/latest/sqlx/)
- [PrimeVue documentation](https://primevue.org/)

## License

This project is licensed under the [MIT License](LICENSE).
