# Local Variables:
# mode: justfile
# End:

set dotenv-load
set export

# Start peripherals of this server
peri:
	docker-compose up -d arangodb

# Development environment preparation.
prepare: peri
	@rustup override set stable
	@if [ ! -f .env ]; then cp .env.example .env; fi
	@if [ ! -f config/main.toml ]; then cp config/main.sample.toml config/main.toml; fi
	@if [ ! -f config/testing.toml ]; then printf "[db]\ndb = \"relation_server_test\"" > config/testing.toml; fi

# Do database migration.
migrate:
	aragog --db-name relation_server_development migrate
	aragog --db-name relation_server_test migrate

# Do aragog CLI command (with .env loaded).
@aragog subcommand subsubcommand:
	aragog $subcommand $subsubcommand

# Run test
test: peri
	env RUST_BACKTRACE=1 RUST_LOG=debug RELATION_SERVER_ENV=testing cargo test -- --nocapture --test-threads=1

# Clean dev environment (incl. build cache and database)
clean:
	cargo clean
	docker-compose down -v

# Get latest schema file. install first: npm install -g get-graphql-schema
get-schema:
    get-graphql-schema https://api.lens.dev/playground > src/upstream/lens/schema.graphql
