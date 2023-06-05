FROM rust:alpine AS builder
ADD . /code
RUN cd /code && \
    apk add musl-dev && \
    cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest
COPY --from=builder /code/target/x86_64-unknown-linux-musl/release/p2s /usr/bin/p2s
