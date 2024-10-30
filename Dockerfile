FROM rust:1.82 AS builder

ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH=$CARGO_HOME/bin:$PATH

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch
COPY src ./src
RUN cargo build --release --bin controller


FROM debian:bookworm-slim

RUN useradd --create-home --uid 1001 user
USER 1001

WORKDIR /home/user
COPY --from=builder /app/target/release/controller .

ENV RUST_LOG=INFO
ENTRYPOINT ["./controller"]
