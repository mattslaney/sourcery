ARG VERSION=latest
FROM ubuntu:${VERSION}

RUN apt-get update && apt-get install -y \
    build-essential \
    git \
    make \
    clang \
    libtool-bin \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src
ENTRYPOINT ["/bin/sh", "-c"]
