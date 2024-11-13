FROM rust:1.82 AS builder

ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH=$CARGO_HOME/bin:$PATH

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY chirpstack-operator ./chirpstack-operator
COPY droperator ./droperator
RUN cargo fetch
RUN cargo build --release --bin controller --package chirpstack-operator

FROM debian:bookworm-slim
LABEL org.opencontainers.image.source=https://github.com/deepshore/chirpstack-operator.git

RUN useradd --create-home --uid 1001 user
USER 1001

WORKDIR /home/user
COPY --from=builder /app/target/release/controller .

ENV RUST_LOG=info
ENTRYPOINT ["./controller"]
