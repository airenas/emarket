#########################################################################################
## docker will invoke this file from ../.. dir in order to access code
#########################################################################################
FROM rust:1.67-buster as builder

# RUN apk --update add openssl-dev musl-dev

ARG BUILD_VERSION=0.1

WORKDIR /src/

COPY emarket /src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
      CARGO_APP_VERSION=$BUILD_VERSION cargo build --release
#########################################################################################
FROM debian:buster-slim as runner

RUN apt-get update && apt-get -y install openssl ca-certificates


ARG BUILD_VERSION=0.1

WORKDIR /app
ENV UID=999

RUN addgroup --gid $UID app && useradd -r -M -u $UID -g app app   
RUN mkdir -p /app && chown -R app:app /app   

COPY --from=builder /src/target/release/importer-ws /app/

RUN chown app:app /app/* /app
 
USER app

ENTRYPOINT ["./importer-ws"]
