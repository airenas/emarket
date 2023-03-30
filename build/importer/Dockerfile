#########################################################################################
## docker will invoke this file from ../.. dir in order to access code
#########################################################################################
FROM rust:1.67-buster as builder

# RUN apk --update add openssl-dev musl-dev

ARG BUILD_VERSION=0.1

WORKDIR /src/

COPY emarket /src

# RUN --mount=type=cache,target=/usr/local/cargo/registry \
#       CARGO_APP_VERSION=$BUILD_VERSION cargo build --release
RUN CARGO_APP_VERSION=$BUILD_VERSION cargo build --release      
#########################################################################################
FROM debian:buster-slim as runner

RUN apt-get update && apt-get -y install openssl ca-certificates


ARG BUILD_VERSION=0.1

LABEL org.opencontainers.image.version=$BUILD_VERSION \
      org.opencontainers.image.authors="airenass@gmail.com" \
      name="emarket-importer" \
      maintainer="airenass@gmail.com" \
      vendor="airenass@gmail.com" \
      version=$BUILD_VERSION \
      release=$BUILD_VERSION \
      summary="This image is used to host the emarket import service" \
      description="This image is used to host the emarket import service" 

WORKDIR /app
ENV UID=999

RUN addgroup --gid $UID app && useradd -r -M -u $UID -g app app   
RUN mkdir -p /app && chown -R app:app /app   

COPY --from=builder /src/target/release/importer /app/

RUN chown app:app /app/* /app
 
USER app

ENTRYPOINT ["./importer"]
