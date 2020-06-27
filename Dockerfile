FROM chainxorg/chainx-base-builder
LABEL maintainer "xuliuchengxlc@gmail.com"
LABEL description="The build stage for ChainX. We create the ChainX binary in this stage."

ARG APP=chainx

WORKDIR /$APP

COPY . /$APP

RUN cargo build --release
