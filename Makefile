.PHONY: help setup db-up db-down db-logs api cli-health seed demo-reset demo fmt fmt-check lint test build web-install web-dev web-lint web-build ci

help:
	@echo "setup      - copy .env.example to .env and install web deps"
	@echo "db-up      - start PostgreSQL via docker compose"
	@echo "db-down    - stop PostgreSQL"
	@echo "api        - run the API (requires db-up)"
	@echo "cli-health - query /api/health through the CLI"
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
	cargo run -p ecoquest-api

cli-health:
	cargo run -p ecoquest-cli -- health

# Reads DATABASE_URL and SEED_* from .env. Lives with the API, not the CLI:
# the CLI never opens a database connection, and the first ADMIN cannot be
# created over HTTP.
seed:
	cargo run -p ecoquest-api --bin seed

# Development/test only. reset destroys all local data.
demo-reset:
	APP_ENV=development powershell -ExecutionPolicy Bypass -File scripts/reset-demo.ps1 -Force

demo:
	APP_ENV=development powershell -ExecutionPolicy Bypass -File scripts/demo.ps1

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
