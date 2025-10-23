ARG VERSION=latest
FROM amazonlinux:${VERSION}

RUN if command -v dnf > /dev/null; then \
        dnf install -y make gcc git clang libtool && dnf clean all; \
    elif command -v yum > /dev/null; then \
        yum install -y make gcc git clang libtool && yum clean all; \
    else \
        echo "No suitable package manager found!"; exit 1; \
    fi

WORKDIR /usr/src
ENTRYPOINT ["/bin/sh", "-c"]
