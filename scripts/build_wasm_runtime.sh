#!/usr/bin/env bash

export RUSTC_VERSION=nightly-2021-03-01
export PACKAGE=chainx-runtime

# Use the toolchain specified in chainxorg/srtool
mv rust-toolchain rust-toolchain.bak

docker run --rm -it -e PACKAGE="$PACKAGE" -e BUILD_OPTS=" " -v $PWD:/build -v /tmp/out:/out -v /tmp/cargo:/cargo-home chainxorg/srtool:$RUSTC_VERSION

mv rust-toolchain.bak rust-toolchain
