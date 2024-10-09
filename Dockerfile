FROM s390x/fedora:latest

RUN dnf install -y git cargo

RUN git clone https://github.com/maciejhirsz/logos /logos

WORKDIR /logos

RUN cargo test --workspace --verbose
