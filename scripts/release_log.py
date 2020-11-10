#!/usr/bin/env python3
if __name__ == '__main__':
    import json
    log = json.load(open('srtool_output.json'))
    print("### build environment  \n{}  \n ### wasm info  \nchainx_runtion hash: {}  \n", log['rustc'], log['sha256'])
