# Local Variables:
# mode: justfile
# End:

set dotenv-load
set export

# Start peripherals of this server
peri:
	docker-compose up -d tigergraph

# Development environment preparation.
prepare: peri
	@rustup override set stable
	@if [ ! -f .env ]; then cp .env.example .env; fi
	@if [ ! -f config/main.toml ]; then cp config/main.sample.toml config/main.toml; fi

# Run tests
test: peri
	env RUST_BACKTRACE=1 RUST_LOG=debug RELATION_SERVER_ENV=testing cargo test -- --nocapture --test-threads=1

# Clean dev environment (incl. build cache and database)
clean:
	cargo clean
	docker-compose down -v

@local:
	docker compose down -v
	docker compose up -d
	echo 'Waiting for TigerGraph to start...'
	sleep 60
	env RUST_LOG=trace cargo run --bin standalone

# Get latest schema file.
# npm install -g get-graphql-schema
get-schema:
    get-graphql-schema https://api.lens.dev/playground > src/upstream/lens/schema.graphql
