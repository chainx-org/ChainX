#!/usr/bin/env python3

if __name__ == '__main__':
    import json
    env = json.load(open('srtool_output.json'))
    message = f"""
WASM runtime built with [srtool](https://hub.docker.com/r/chainxorg/srtool) using `{env['rustc']}`.

chainx runtime proposal hash: `{env['prop']}`.

#### Changes

##### Runtime

"""
    print(message)
