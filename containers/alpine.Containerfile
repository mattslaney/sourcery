ARG VERSION=latest
FROM alpine:${VERSION}

RUN apk add --no-cache \
    build-base \
    git \
    clang \
    libtool

WORKDIR /usr/src
ENTRYPOINT ["/bin/sh", "-c"]
