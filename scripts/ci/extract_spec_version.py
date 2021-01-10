#!/usr/bin/env python
# -*- coding: utf-8 -*-

import os
import json

def main():
    # Change the working directory to project root directory.
    os.chdir("../..")

    f = open("runtime/chainx/src/lib.rs")
    for line in f.readlines():
        if line.strip().startswith('spec_version'):
            version = ([int(s) for s in line.strip()[:-1].split() if s.isdigit()])
            with open("spec_version.json", 'w') as outfile:
                if not version:
                    json.dump({ 'version': '?' }, outfile)
                else:
                    json.dump({ 'version': version[0] }, outfile)
            break

main()
