FROM debian:9.7-slim

RUN apt update && apt install -y libssl1.1
WORKDIR /usr/src/app
COPY ./target/release .
RUN mkdir -p config
COPY config.json ./config

CMD ["./rust-crm-sync"]
