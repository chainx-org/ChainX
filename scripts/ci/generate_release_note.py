#!/usr/bin/env python3

if __name__ == '__main__':
    import json
    env = json.load(open('srtool_output.json'))
    message = f"""
WASM runtime built with [srtool](TODO: link to docker image) using `{env['rustc']}`.

chainx runtime proposal hash: `{env['prop']}`.

#### Changes
    """
    print(message)
