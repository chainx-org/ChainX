#!/usr/bin/env bash

RUSTC_VERSION=nightly-2021-03-01
PACKAGE=chainx-runtime

# Use the toolchain specified in chainxorg/srtool
if [ -f rust-toolchain ]; then
  mv rust-toolchain rust-toolchain.bak
fi

docker run --rm -it -e PACKAGE="$PACKAGE" -e BUILD_OPTS=" " -v $PWD:/build -v /tmp/out:/out -v /tmp/cargo:/cargo-home chainxorg/srtool:$RUSTC_VERSION

if [ -f rust-toolchain.bak ]; then
  mv rust-toolchain.bak rust-toolchain
fi
