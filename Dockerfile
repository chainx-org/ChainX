FROM docker.io/library/ubuntu:20.04

ARG CI_GIT_TAG
ARG CI_GIT_SHA
ARG CI_BUILD_AT

# See:
# https://github.com/opencontainers/image-spec/blob/main/annotations.md
# https://github.com/paritytech/polkadot/blob/master/scripts/dockerfiles/polkadot_injected_release.Dockerfile
LABEL org.chainx.image.created="${CI_BUILD_AT}" \
    org.chainx.image.authors="icodezjb@gmail.com" \
    org.chainx.image.url="https://github.com/chainx-org/ChainX" \
    org.chainx.image.documentation="https://chainx-org.github.io/documentation/" \
    org.chainx.image.source="https://github.com/chainx-org/ChainX" \
    org.chainx.image.version="${CI_GIT_TAG}" \
    org.chainx.image.revision="${CI_GIT_SHA}" \
    org.chainx.image.licenses="GPL-3.0" \
    org.chainx.image.title="ChainX" \
    org.chainx.image.description="BTC Layer2 & Hubs for multi-chain systems such as MiniX/SherpaX & Backend chain hub of ComingChat."

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    update-ca-certificates

COPY shared/chainx /usr/local/bin/chainx

RUN mkdir -p /root/.local/share/chainx && \
    ln -s /root/.local/share/chainx /data && \
# Sanity checks
    ldd /usr/local/bin/chainx && \
    /usr/local/bin/chainx --version

EXPOSE 30333 8086 8087 9615

VOLUME ["/data"]

CMD ["/usr/local/bin/chainx"]
