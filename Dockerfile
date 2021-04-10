FROM rust:1.51.0-buster as builder
COPY . /usr/src/login_server/
WORKDIR /usr/src/login_server/

RUN cargo install --path .


FROM ubuntu:focal

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && apt-get install -y \
    ca-certificates \
    libssl-dev

COPY --from=builder /usr/local/cargo/bin/login_server /usr/local/bin/login_server

ENV USE_ENVIRONMENTAL_VARIABLES=TRUE
EXPOSE 8080
CMD [ "sh", "-c", "/usr/local/bin/login_server" ]
