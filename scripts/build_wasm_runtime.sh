#!/usr/bin/env bash

# Make sure the script is running from the root directory of ChainX.
cd "$(dirname "${BASH_SOURCE[0]}")"
cd ..

RUSTC_VERSION=nightly-2021-03-01
PACKAGE=chainx-runtime

# Use the toolchain specified in chainxorg/srtool instead of the one in ChainX
if [ -f rust-toolchain ]; then
  mv rust-toolchain rust-toolchain.bak
fi

docker run --rm -it -e PACKAGE="$PACKAGE" -e BUILD_OPTS=" " -v $PWD:/build -v /tmp/out:/out -v /tmp/cargo:/cargo-home chainxorg/srtool:$RUSTC_VERSION

if [ -f rust-toolchain.bak ]; then
  mv rust-toolchain.bak rust-toolchain
fi
