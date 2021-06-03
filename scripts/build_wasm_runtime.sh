#!/usr/bin/env bash

# Make sure the script is running from the root directory of ChainX.
cd "$(dirname "${BASH_SOURCE[0]}")"
cd ..

RUSTC_VERSION=nightly-2021-03-01
PACKAGE=chainx-runtime

timestamp=$(date +%s)
toolchain_backup=rust-toolchain.bak."$timestamp"

# Use the toolchain specified in chainxorg/srtool instead of the one in ChainX
if [ -f rust-toolchain ]; then
  mv rust-toolchain "$toolchain_backup"
fi

docker run --rm -it -e PACKAGE="$PACKAGE" -e BUILD_OPTS=" " -v $PWD:/build -v /tmp/out:/out -v /tmp/cargo:/cargo-home chainxorg/srtool:$RUSTC_VERSION

if [ -f "$toolchain_backup" ]; then
  mv "$toolchain_backup" rust-toolchain
fi
