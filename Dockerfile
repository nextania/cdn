FROM rust:1.92.0 AS builder
USER 0:0
WORKDIR /usr/app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo install --path . --locked
 
FROM debian:trixie-slim
WORKDIR /usr/app
RUN apt update && apt install -y ca-certificates
COPY --from=builder /usr/local/cargo/bin/cdn ./
CMD ["./cdn"]