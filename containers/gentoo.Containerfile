ARG VERSION=latest
FROM gentoo:${VERSION}

RUN emerge --sync && \
    emerge -y sys-devel/gcc dev-vcs/git sys-devel/make sys-devel/clang sys-devel/libtool

WORKDIR /usr/src
ENTRYPOINT ["/bin/bash"]
