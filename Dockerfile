FROM phusion/baseimage:0.10.1 as builder
LABEL maintainer "xuliuchengxlc@gmail.com"
LABEL description="The build stage for ChainX. We create the ChainX binary in this stage."

ARG PROFILE=release
ARG APP=chainx

WORKDIR /$APP

COPY . /$APP

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y cmake pkg-config libssl-dev git clang libclang-dev

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    export PATH=$PATH:$HOME/.cargo/bin && \
    rustup toolchain install nightly-x86_64-unknown-linux-gnu && \
    cargo +nightly build --$PROFILE

# ===== SECOND STAGE ======

FROM phusion/baseimage:0.10.0
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

EXPOSE 30333 8086 8087

VOLUME ["/data"]

CMD ["/usr/local/bin/chainx"]
