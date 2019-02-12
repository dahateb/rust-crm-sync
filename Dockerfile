FROM debian:9.7-slim

WORKDIR /usr/src/app
COPY ./target .
RUN mkdir -p config
COPY ./deploy/config.json ./config

CMD ["/usr/src/app/target/release/rust-crm-sync"]
