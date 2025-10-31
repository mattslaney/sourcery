ARG VERSION=latest
FROM archlinux:${VERSION}

RUN pacman -Sy --noconfirm \
    base-devel \
    git \
    clang \
    libtool \
    && pacman -Scc --noconfirm

WORKDIR /usr/src
ENTRYPOINT ["/bin/sh", "-c"]
