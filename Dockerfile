FROM rust:1.51.0-buster as builder
COPY . /usr/src/twinsight_content_dashboard/
WORKDIR /usr/src/twinsight_content_dashboard/

RUN cargo install --path .


FROM ubuntu:focal

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && apt-get install -y \
    ca-certificates \
    libssl-dev

COPY --from=builder /usr/local/cargo/bin/twinsight_content_dashboard /usr/local/bin/twinsight_content_dashboard

ENV USE_ENVIRONMENTAL_VARIABLES=TRUE
EXPOSE 8080
CMD [ "sh", "-c", "/usr/local/bin/twinsight_content_dashboard" ]
