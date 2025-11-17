# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.90.0

FROM --platform=$BUILDPLATFORM rust:${RUST_VERSION} AS build
WORKDIR /app
RUN apt-get update && apt-get install -y sccache lld
ENV RUSTC_WRAPPER=sccache
ENV RUSTFLAGS="-C link-arg=-fuse-ld=lld"
RUN --mount=type=bind,source=crates,target=crates \
  --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
  --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
  --mount=type=cache,target=/app/target/ \
  --mount=type=cache,target=/usr/local/cargo/git/db \
  --mount=type=cache,target=/usr/local/cargo/registry/cache \
  cargo build --locked --release && \
  cp ./target/release/0ae /bin/0ae && \
  cp ./target/release/0api /bin/0api

FROM debian:stable AS final
RUN apt-get update \
  && apt-get install --no-install-recommends -y libssl3t64=3.5.* ca-certificates=20250419 \
  iproute2=6.15.0-1 caddy=2.6.2-12+b3 netcat-openbsd=1.229-1 pkg-config=1.8.1-4 curl \
  && rm -rf /var/lib/apt/lists/*
ARG UID=10001
RUN useradd \
  --create-home \
  --home-dir /usr/share/nullae \
  --shell /sbin/nologin \
  --uid "${UID}" \
  --comment "" \
  nullae
USER nullae
COPY --from=build /bin/0ae /bin/
COPY --from=build /bin/0api /bin/
EXPOSE 3000
WORKDIR /usr/share/nullae
CMD ["/bin/nullae"]
