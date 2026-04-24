# stadera-storage

Postgres persistence layer. Repositories for the domain aggregates.

## Local dev setup

Prerequisites:

- Docker (Compose v2)
- `sqlx-cli`: `cargo install sqlx-cli --no-default-features --features postgres,rustls`

Bring up Postgres and apply migrations:

```bash
cp .env.example .env  # adjust if needed
make db-up
make db-migrate
```
