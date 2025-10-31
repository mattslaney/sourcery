ARG VERSION=latest
FROM centos:${VERSION}

RUN yum install -y \
    make \
    gcc \
    git \
    clang \
    libtool \
    && yum clean all

WORKDIR /usr/src
ENTRYPOINT ["/bin/sh", "-c"]
