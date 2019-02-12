FROM debian:9.7-slim

RUN apt update && apt install -y libssl1.1
WORKDIR /usr/src/app
COPY ./target .
RUN mkdir -p release/config
COPY config.json ./release/config

CMD ["cd /usr/src/app/release && .rust-crm-sync"]
