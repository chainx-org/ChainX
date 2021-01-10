#!/usr/bin/env python3

if __name__ == '__main__':
    import json
    env = json.load(open('srtool_output.json'))
    spec_version = json.load(open('spec_version.json'))
    message = f"""
Upgrade priority:

- [ ] **Medium** (upgrade at your earliest convenience)
- [ ] **High** (upgrade ASAP)

WASM runtime built with [srtool](https://hub.docker.com/r/chainxorg/srtool) using `{env['rustc']}`.

chainx runtime proposal hash: `{env['prop']}`.

Native runtime version: `{spec_version['version']}`

#### Changes

##### Runtime

##### Others
"""
    print(message)
