.PHONY: help db-up db-down db-reset db-migrate db-psql check pair sync

# Default user-email for pair / sync. Override with: make pair USER_EMAIL=alice@example.com
USER_EMAIL ?= michelebellitti78@gmail.com

help:
	@echo "Dev targets:"
	@echo "  db-up        Start Postgres (Docker Compose)"
	@echo "  db-down      Stop Postgres"
	@echo "  db-reset     Destroy volume and recreate + migrate"
	@echo "  db-migrate   Run sqlx migrations against local DB"
	@echo "  db-psql      Open psql shell into local DB"
	@echo "  pair         Run first-time Withings OAuth pairing (USER_EMAIL=...)"
	@echo "  sync         Run a one-shot Withings sync (USER_EMAIL=...)"
	@echo "  check        cargo fmt + clippy + test"

db-up:
	docker compose up -d postgres
	@echo "Waiting for Postgres to be ready..."
	@until docker compose exec -T postgres pg_isready -U stadera -d stadera > /dev/null 2>&1; do sleep 1; done
	@echo "Postgres ready at postgres://stadera:stadera@localhost:5432/stadera"

db-down:
	docker compose down

db-reset:
	docker compose down -v
	$(MAKE) db-up
	$(MAKE) db-migrate

db-migrate:
	cd crates/storage && sqlx migrate run

db-psql:
	docker compose exec postgres psql -U stadera -d stadera

pair:
	cargo run -p stadera-withings --bin pair -- --user-email $(USER_EMAIL)

sync:
	cargo run -p stadera-jobs -- sync --user-email $(USER_EMAIL)

check:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test --all