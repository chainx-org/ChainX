#!/usr/bin/env python3

if __name__ == '__main__':
    import json
    env = json.load(open('srtool_output.json'))
    message = f"""
Upgrade priority:

- [ ] **Medium** (upgrade at your earliest convenience)
- [ ] **High** (upgrade ASAP)

WASM runtime built with [srtool](https://hub.docker.com/r/chainxorg/srtool) using `{env['rustc']}`.

chainx runtime proposal hash: `{env['prop']}`.

Native runtime version: **?**

#### Changes

##### Runtime

##### Others
"""
    print(message)
