# This is the build stage for ChainX. Here we create the binary.
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /chainx
COPY . /chainx
RUN cargo build --locked --release

# This is the 2nd stage: a very small image where we copy the ChainX binary."
FROM docker.io/library/ubuntu:20.04

COPY --from=builder /chainx/target/release/chainx /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /chainx chainx && \
    mkdir -p /data /chainx/.local/share/chainx && \
    chown -R chainx:chainx /data && \
    ln -s /data /chainx/.local/share/chainx && \
# unclutter and minimize the attack surface
    rm -rf /usr/bin /usr/sbin && \
# Sanity checks
    ldd /usr/local/bin/chainx && \
    /usr/local/bin/chainx --version

USER chainx
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

CMD ["/usr/local/bin/chainx"]
