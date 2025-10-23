ARG VERSION=latest
FROM opensuse/leap:${VERSION}

RUN zypper install -y \
    make \
    gcc \
    git \
    clang \
    libtool \
    && zypper clean --all

WORKDIR /usr/src
ENTRYPOINT ["/bin/sh", "-c"]
