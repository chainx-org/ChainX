FROM phusion/baseimage:0.11 as builder
LABEL maintainer "xuliuchengxlc@gmail.com"
LABEL description="The build stage for ChainX. We create the ChainX binary in this stage."

ARG PROFILE=release
ARG APP=chainx
ARG RUSTC_VERSION=nightly-2021-03-01

WORKDIR /$APP

COPY . /$APP

RUN apt-get update && \
    apt-get -o Dpkg::Options::="--force-confdef" -o Dpkg::Options::="--force-confold" dist-upgrade -y && \
    apt-get install -y cmake pkg-config libssl-dev git clang

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    export PATH=$PATH:$HOME/.cargo/bin && \
    rustup toolchain install $RUSTC_VERSION && \
    rustup target add wasm32-unknown-unknown --toolchain $RUSTC_VERSION && \
    cargo +$RUSTC_VERSION build --$PROFILE

# ===== SECOND STAGE ======

FROM phusion/baseimage:0.11
LABEL maintainer "xuliuchengxlc@gmail.com"
LABEL description="A very small image where we copy the ChainX binary created from the builder image."

ARG PROFILE=release
ARG APP=chainx

COPY --from=builder /$APP/target/$PROFILE/$APP /usr/local/bin

RUN mv /usr/share/ca* /tmp && \
    rm -rf /usr/share/*  && \
    mv /tmp/ca-certificates /usr/share/ && \
    rm -rf /usr/lib/python* && \
    mkdir -p /root/.local/share/chainx && \
    ln -s /root/.local/share/chainx /data

RUN rm -rf /usr/bin /usr/sbin

EXPOSE 20222 8086 8087

VOLUME ["/data"]

CMD ["/usr/local/bin/chainx"]
