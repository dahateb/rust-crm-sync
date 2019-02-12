FROM debian:9.7

WORKDIR /usr/src/app
COPY ./target .
RUN mkdir -p release/config
COPY config.json ./release/config

CMD ["/usr/src/app/release/rust-crm-sync"]
