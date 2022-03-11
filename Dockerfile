FROM docker.io/library/ubuntu:20.04
LABEL maintainer "icodezjb@gmail.com"
LABEL description="An image where we copy the ChainX binary created from the builder image."

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    update-ca-certificates

COPY shared/chainx /usr/local/bin/chainx

RUN mkdir -p /root/.local/share/chainx && \
    ln -s /root/.local/share/chainx /data \
# Sanity checks
    ldd /usr/local/bin/chainx && \
    /usr/local/bin/chainx --version

EXPOSE 30333 8086 8087 9615

VOLUME ["/data"]

CMD ["/usr/local/bin/chainx"]
