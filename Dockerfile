FROM clux/muslrust:nightly

WORKDIR /build
ADD Cargo.toml Cargo.lock /build/
RUN mkdir /build/src
RUN echo 'fn main() {}' > src/main.rs

RUN cargo fetch
ADD src /build/src
RUN cargo build --release
RUN mkdir /artifacts
RUN mv target/x86_64-unknown-linux-musl/release/airmash-ground-control /artifacts/airmash-ground-control

FROM alpine:3.8
COPY --from=0 /artifacts/airmash-ground-control /airmash-ground-control
ENV RUST_LOG="airmash_ground_control=info"

ENTRYPOINT ["./airmash-ground-control"]
CMD ["wss://us.airmash.online/ffa2", "wss://eu.airmash.online/ffa2"]