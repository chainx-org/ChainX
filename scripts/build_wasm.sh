#!/usr/bin/env bash
docker run --rm -it -e PACKAGE=chainx-runtime -v $PWD:/build -v /tmp/cargo:/cargo-home chainxorg/srtool:nightly-2020-09-30
