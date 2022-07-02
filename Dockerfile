# Build standalone Docker image
FROM rust:buster AS builder

WORKDIR /app

# Stupid, I know.
# SEE ALSO: https://github.com/rust-lang/cargo/issues/2644
RUN mkdir src && touch src/lib.rs
ADD Cargo.toml .
ADD Cargo.lock .
RUN cargo build --release && rm -r src

ADD . .
RUN cargo build --release --example standalone && strip target/release/examples/standalone

# =-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-
FROM debian:buster AS runner
LABEL maintainer="Nyk Ma <nykma@mask.io>"

WORKDIR /app

COPY --from=builder /app/target/release/examples/standalone /app/server

RUN chmod a+x server && \
    mkdir config && \
    apt-get update && \
    apt-get install -y openssl && \
    rm -rf /var/lib/apt

VOLUME ["/app/config"]

EXPOSE 8000

CMD ["/app/server"]
