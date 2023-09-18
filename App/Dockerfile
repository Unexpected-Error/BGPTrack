FROM rust:latest as build

WORKDIR /usr/src/App
COPY . .
RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres
RUN cargo build --release

FROM gcr.io/distroless/cc-debian10

COPY --from=build /usr/src/App/target/release/bgp_track /usr/local/bin/bgp_track
WORKDIR /usr/local/bin

#E NV SQLX_OFFLINE=true
# CMD ["bgp_track -v find-shortlived"]