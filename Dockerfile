####################################################################################################
## Builder
####################################################################################################
FROM rust:latest AS builder

RUN rustup target add aarch64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

# Create appuser
ENV USER=wfeusk
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

WORKDIR /wfeusk

COPY ./ .

RUN cargo build --target aarch64-unknown-linux-musl --release
RUN objdump -d target/aarch64-unknown-linux-musl/release/wfeusk >wfeusk.asm
