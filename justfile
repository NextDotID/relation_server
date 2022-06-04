# Local Variables:
# mode: justfile
# End:

arch := `uname -m`

# Development environment preparation
prepare:
	rustup override set stable
	docker-compose pull arangodb_{{arch}}
	cp config/main.sample.toml config/main.toml
	echo '[db]\ndb = "relation_server_test"' > config/test.toml

# Do database migration
migrate:
	docker-compose up -d arangodb_{{arch}}
	aragog --db-host http://127.0.0.1:8529 --db-user root --db-password ieNgoo5roong9Chu --db-name relation_server_development migrate
	aragog --db-host http://127.0.0.1:8529 --db-user root --db-password ieNgoo5roong9Chu --db-name relation_server_test migrate

# Run test
test:
	env RUST_BACKTRACE=1 RUST_LOG=debug cargo test -- --nocapture --test-threads=1

# Clean dev environment (incl. build cache and database)
clean:
	cargo clean
	docker-compose down -v
