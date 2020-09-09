#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import sys

import toml

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
        if 'contracts' in member:
            print('Skipped bump version for package', member)
            continue
        cargo_path = os.path.join(member, 'Cargo.toml')
        do_bump_version(cargo_path)

    do_bump_version("Cargo.toml")


main()
