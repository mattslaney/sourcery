ARG VERSION=latest
FROM fedora:${VERSION}

RUN dnf install -y \
    make \
    gcc \
    git \
    clang \
    libtool \
    && dnf clean all

WORKDIR /usr/src
ENTRYPOINT ["/bin/sh", "-c"]
