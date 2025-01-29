#########################################################################################
## docker will invoke this file from ../.. dir in order to access code
#########################################################################################
FROM rust:1.84-bullseye AS builder

# RUN apk --update add openssl-dev musl-dev

ARG BUILD_VERSION=0.1

WORKDIR /src/

COPY emarket /src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
      CARGO_APP_VERSION=$BUILD_VERSION cargo build --release

#########################################################################################
FROM gcr.io/distroless/cc-debian11 AS ssl

#########################################################################################
# Debian 12 does not include ssl libs
FROM gcr.io/distroless/cc-debian12 AS runner
#########################################################################################

COPY LICENSE /licenses/LICENSE-gnu

WORKDIR /app

### /ssl
COPY --from=ssl /usr/lib/x86_64-linux-gnu/libssl.so.* /lib/x86_64-linux-gnu/
COPY --from=ssl /usr/lib/x86_64-linux-gnu/libcrypto.so.* /lib/x86_64-linux-gnu/
#########################################################################################

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

COPY --from=builder /src/target/release/importer /app/

ENTRYPOINT ["/app/importer"]
