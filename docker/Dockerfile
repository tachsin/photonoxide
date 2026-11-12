# syntax=docker/dockerfile:1
#
# The photonoxide program without its window: every headless command (run --headless, worker,
# new, schema, export, example, validate, bench, libraries), for laptops, cloud machines, CI and
# clusters (through Apptainer). docs/docker.md says how to use it; docker/README.md is its page on
# Docker Hub.
#
# Built from the repository's root:
#
#   docker build -f docker/Dockerfile -t photonoxide .
#
# The builder is the Rust that rust-toolchain.toml pins: RUST_VERSION must be that version, and
# the build fails if it isn't (CI reads it from the file). Debian slim with glibc, not static musl:
# Rust's f64 functions (sin, exp, ...) call the platform's libm, and musl's could change the bits
# against the Linux packages. CI checks the image's results against a native run's, to the bit.

ARG RUST_VERSION=1.100.0
ARG DEBIAN=trixie

FROM rust:${RUST_VERSION}-slim-${DEBIAN} AS builder
ARG RUST_VERSION
# what the program says it was built from (`photonoxide --version`): the context has no .git
ARG PHOTONOXIDE_COMMIT=""
ARG PHOTONOXIDE_COMMIT_DATE=""
# fewer parallel compiles on a small machine (empty: all its cores)
ARG CARGO_BUILD_JOBS=""
WORKDIR /src
COPY . .
# this image's Rust, never the one rust-toolchain.toml would make rustup fetch (with components
# the build doesn't need); and it must be the pinned one
ENV RUSTUP_TOOLCHAIN=${RUST_VERSION}
RUN pinned=$(sed -n 's/^channel *= *"\([^"]*\)".*/\1/p' rust-toolchain.toml) \
    && if [ "$pinned" != "$RUST_VERSION" ]; then \
         echo "rust-toolchain.toml pins Rust $pinned, this builder is $RUST_VERSION: build with --build-arg RUST_VERSION=$pinned" >&2; \
         exit 1; \
       fi
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    if [ -n "$CARGO_BUILD_JOBS" ]; then export CARGO_BUILD_JOBS; else unset CARGO_BUILD_JOBS; fi \
    && PHOTONOXIDE_COMMIT="$PHOTONOXIDE_COMMIT" PHOTONOXIDE_COMMIT_DATE="$PHOTONOXIDE_COMMIT_DATE" \
       cargo build --locked --release -p photonoxide-studio --no-default-features \
    && cp target/release/photonoxide /usr/local/bin/photonoxide

FROM debian:${DEBIAN}-slim
ARG VERSION=""
ARG PHOTONOXIDE_COMMIT=""
ARG CREATED=""
LABEL org.opencontainers.image.title="photonoxide" \
      org.opencontainers.image.description="Photonics design and simulation: the photonoxide program without its window, for runs, farm workers and checks" \
      org.opencontainers.image.source="https://github.com/tachsin/photonoxide" \
      org.opencontainers.image.url="https://github.com/tachsin/photonoxide" \
      org.opencontainers.image.documentation="https://github.com/tachsin/photonoxide/blob/main/docs/docker.md" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0" \
      org.opencontainers.image.vendor="tachsin" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${PHOTONOXIDE_COMMIT}" \
      org.opencontainers.image.created="${CREATED}"
# ca-certificates alone: the program links nothing else beyond glibc; references --crossref uses
# curl, which isn't here (it is a maintainers' command)
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/bin/photonoxide /usr/local/bin/photonoxide
COPY LICENSE-MIT LICENSE-APACHE /usr/share/doc/photonoxide/
# a user of its own, not root; `docker run --user "$(id -u):$(id -g)"` runs as yours, so the
# files a run writes into a mounted folder are yours
RUN useradd --uid 1000 --user-group --create-home --shell /usr/sbin/nologin photonoxide \
    && mkdir -p /work && chown photonoxide:photonoxide /work
USER photonoxide
WORKDIR /work
# the program itself is PID 1: it handles SIGTERM (docker stop) and SIGINT, so no init is needed
ENTRYPOINT ["photonoxide"]
