#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import sys

try:
    import toml
except ImportError as e:
    print(
        'Please first install toml using `pip3 install toml` and rerun this script'
    )
    exit(1)

if len(sys.argv) != 2:
    print('  Usage: ./apply_bump_version.py next_version')
    print('Example: ./apply_bump_version.py 2.0.0')
    exit(1)

next_version = sys.argv[1]


def read_file(fname):
    f = open(fname)
    return f.readlines()


def write_back(lines, f):
    with open(f, 'w') as writer:
        writer.writelines(lines)


def do_bump_version(cargo_path):
    lines = read_file(cargo_path)
    #  We assume all Cargo.toml follows the auto generated template, otherwise
    #  skip it.
    if lines[0].strip() != '[package]':
        print(cargo_path, 'may not be bumped properly, skipping...')
        return

    for idx in range(1, len(lines)):
        if lines[idx].startswith('version'):
            lines[idx] = 'version = "{version}"\n'.format(version=next_version)
            write_back(lines, cargo_path)
            break


def main():
    # Change the working directory to project root directory.
    os.chdir("../")

    for member in toml.load("Cargo.toml")['workspace']['members']:
        #  Ignore the forked contracts pallets
        if 'contracts' in member:
            print('Skipped bump version for package', member)
            continue

        cargo_path = os.path.join(member, 'Cargo.toml')
        do_bump_version(cargo_path)

    do_bump_version("Cargo.toml")


main()
