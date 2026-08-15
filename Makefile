.PHONY: help setup db-up db-down db-logs api cli-health migrate seed reset demo-reset demo verify-flow fmt fmt-check lint test build web-install web-dev web-lint web-build ci

help:
	@echo "setup      - copy .env.example to .env and install web deps"
	@echo "db-up      - start PostgreSQL via docker compose"
	@echo "db-down    - stop PostgreSQL"
	@echo "api        - run the API (requires db-up)"
	@echo "cli-health - query /api/health through the CLI"
	@echo "migrate    - apply pending database migrations"
	@echo "seed       - create accounts and load the full demo dataset"
	@echo "reset      - wipe all data (drop schema), migrate and re-seed"
	@echo "verify-flow - walk the whole user flow against a running API"
	@echo "fmt/lint/test/build - Rust checks"
	@echo "web-dev/web-lint/web-build - frontend tasks"
	@echo "ci         - everything CI runs"

setup:
	cp -n .env.example .env || true
	cd web && npm install

db-up:
	docker compose up -d postgres

db-down:
	docker compose down

db-logs:
	docker compose logs -f postgres

api:
	cargo run -p ecoquest-api --bin ecoquest-api

cli-health:
	cargo run -p ecoquest-cli -- health

# Direct database access; no API required.
migrate:
	cargo run -p ecoquest-cli -- migrate

# Reads DATABASE_URL and SEED_* from .env. Idempotent: re-running never
# overwrites existing accounts or demo data.
seed:
	cargo run -p ecoquest-api --bin seed

# Development/test only. reset destroys all local data.
reset:
	docker compose exec postgres psql -U ecoquest -d ecoquest -c "DROP SCHEMA public CASCADE"
	docker compose exec postgres psql -U ecoquest -d ecoquest -c "CREATE SCHEMA public"
	docker compose exec postgres psql -U ecoquest -d ecoquest -c "GRANT ALL ON SCHEMA public TO ecoquest"
	$(MAKE) migrate
	$(MAKE) seed

# Development/test only. legacy Windows helpers.
demo-reset:
	APP_ENV=development powershell -ExecutionPolicy Bypass -File scripts/reset-demo.ps1 -Force

demo:
	APP_ENV=development powershell -ExecutionPolicy Bypass -File scripts/demo.ps1

# End-to-end acceptance walk of the documented user flow. Requires a running
# API and database; development/test only (it grants ADMIN via the database).
verify-flow:
	bash scripts/verify-user-flow.sh

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

build:
	cargo build --workspace --release

web-install:
	cd web && npm install

web-dev:
	cd web && npm run dev

web-lint:
	cd web && npm run lint

web-build:
	cd web && npm run build

ci: fmt-check lint test web-lint web-build
