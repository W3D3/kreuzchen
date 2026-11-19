FROM rust:1 AS build
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates/engine crates/engine
COPY crates/registry crates/registry
# The workspace also lists the Tauri app, which the server does not need. Without --locked because
# dropping that member prunes Cargo.lock; existing pins are kept.
RUN sed -i 's|^members = .*|members = ["crates/engine", "crates/registry"]|' Cargo.toml \
 && grep -q 'members = \["crates/engine", "crates/registry"\]' Cargo.toml \
 && cargo build --release --target x86_64-unknown-linux-musl -p kreuzchen-registry \
 && mkdir /data-empty

FROM gcr.io/distroless/static:nonroot
COPY --from=build /src/target/x86_64-unknown-linux-musl/release/kreuzchen-registry /kreuzchen-registry
# Owned by nonroot so a fresh named volume is writable for admin uploads. Bind mounts keep the
# host's ownership: chown them to 65532 (or mount :ro, which keeps the server read-only).
COPY --from=build --chown=65532:65532 /data-empty /data
USER nonroot
EXPOSE 8080
VOLUME /data
ENTRYPOINT ["/kreuzchen-registry"]
CMD ["serve"]
