# Build standalone Docker image
FROM docker.io/rust:buster AS builder

WORKDIR /app

ADD . .
RUN cargo build --bins --release && strip target/release/standalone

# =-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-
FROM docker.io/debian:buster AS runner
LABEL maintainer="Nyk Ma <nykma@mask.io>"

WORKDIR /app

COPY --from=builder /app/target/release/standalone /app/server

RUN chmod a+x server && \
    mkdir config && \
    apt-get update && \
    apt-get install -y openssl ca-certificates && \
    rm -rf /var/lib/apt

VOLUME ["/app/config"]

EXPOSE 8000

CMD ["/app/server"]
