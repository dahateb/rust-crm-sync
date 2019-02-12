FROM debian:9.7-slim

WORKDIR /usr/src/app
COPY ./target .


CMD ["/usr/src/app/target/release/rust-crm-sync"]
