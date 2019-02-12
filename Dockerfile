FROM rust:1.32

WORKDIR /usr/src/app
COPY . .

RUN cargo install --path .

CMD ["/usr/src/app/target/release/rust-crm-sync"]
