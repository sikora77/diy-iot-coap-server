FROM rust:alpine
LABEL authors="sikora"

WORKDIR /usr/src/diy-iot
COPY . .
# RUN cargo install --path .
RUN apk add build-base
RUN apk add openssl-dev perl
RUN cargo build --release
EXPOSE 8000

CMD ["ash"]