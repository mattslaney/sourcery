ARG VERSION=latest
FROM ubuntu:${VERSION}

RUN apt-get update && apt-get install -y \
    build-essential \
    git \
    make \
    clang \
    libtool-bin \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Rust system-wide to /usr/local
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path \
    && chmod -R a+w /usr/local/cargo /usr/local/rustup

WORKDIR /usr/src
ENTRYPOINT ["/bin/sh", "-c"]
