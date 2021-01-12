#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import json


def main():
    #  Switch the working directory to project root directory.
    cur_file = os.path.abspath(__file__)
    ci_dir = os.path.dirname(cur_file)
    scripts_dir = os.path.dirname(ci_dir)
    chainx_dir = os.path.dirname(scripts_dir)
    os.chdir(chainx_dir)

    f = open("runtime/chainx/src/lib.rs")
    for line in f.readlines():
        if line.strip().startswith('spec_version'):
            version = ([
                int(s) for s in line.strip()[:-1].split() if s.isdigit()
            ])
            with open("spec_version.json", 'w') as outfile:
                if not version:
                    spec_version = {'version': '?'}
                else:
                    spec_version = {'version': version[0]}
                json.dump(spec_version, outfile)
                print(spec_version)
            break


main()
