ARG VERSION=latest
FROM slackware:${VERSION}

RUN /usr/bin/slackpkg update && \
    /usr/bin/slackpkg install make gcc git clang libtool

WORKDIR /usr/src
ENTRYPOINT ["/bin/sh", "-c"]
