# Multi-stage Rust build for the stadera workspace.
#
# Stages:
#   1. chef    — install cargo-chef so dep compilation can be cached
#                independently from the rest of the source tree.
#   2. planner — generate `recipe.json`, the dependency-graph manifest
#                cargo-chef uses as cache key.
#   3. builder — build all transitive deps once (`cargo chef cook`),
#                then build the workspace binaries against the cached
#                deps. Without cargo-chef, every source change would
#                invalidate the whole `target/` and re-build axum, sqlx,
#                reqwest, oauth2, etc. — turning every CI run into a
#                5-minute build.
#   4. runtime — distroless cc-debian12: no shell, no apt, ~25 MB base.
#                Just glibc + libssl + ca-certs, which is exactly what
#                a Rust binary compiled against bookworm-slim needs.
#
# Both binaries (`stadera-api`, `stadera-jobs`) end up in the final image.
# CMD defaults to `stadera-api`; the Cloud Run Job for sync (M7-step2)
# overrides CMD to `stadera-jobs sync`.

ARG RUST_VERSION=1.89
ARG DEBIAN_VERSION=bookworm

# ---- chef -------------------------------------------------------------
# Pre-built image with cargo-chef already installed — saves ~2 min per
# cold build vs `cargo install cargo-chef`, and bypasses crates.io TLS
# during the chef stage. Tag tracks rust-toolchain.toml.
FROM lukemathwalker/cargo-chef:latest-rust-${RUST_VERSION}-slim-${DEBIAN_VERSION} AS chef
WORKDIR /app

# ---- planner ----------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---- builder ----------------------------------------------------------
FROM chef AS builder

# Build dependencies first. Cached as a single layer keyed on
# recipe.json — only invalidated when Cargo.lock or workspace deps
# change.
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Now bring in the actual source and build only the binaries we ship.
COPY . .
RUN cargo build --release --bin stadera-api --bin stadera-jobs

# ---- runtime ----------------------------------------------------------
# `cc` flavor of distroless: glibc + libgcc + libssl + ca-certs.
# `nonroot` user (uid 65532) is pre-created. No shell — debugging via
# Cloud Logging instead of `kubectl exec`.
FROM gcr.io/distroless/cc-debian12 AS runtime

COPY --from=builder /app/target/release/stadera-api /usr/local/bin/stadera-api
COPY --from=builder /app/target/release/stadera-jobs /usr/local/bin/stadera-jobs
# Migrations bundled for future use (Cloud Run Job that applies them).
COPY --from=builder /app/crates/storage/migrations /app/migrations

USER nonroot

# Cloud Run injects $PORT at runtime; default to 8080 for `docker run`.
ENV PORT=8080
EXPOSE 8080

CMD ["/usr/local/bin/stadera-api"]
