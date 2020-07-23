#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import subprocess
import json
import os
import pprint

wd = os.getcwd()
os.chdir("..")

TARGET_KINDS = ['typedef', 'enum', 'struct']

ALIAS = {'Vec<u8>': 'Text'}

NEW_TYPES = [
    "AssetType",
    "SignedBalance",
    "Chain",
    "AddrStr",
    "Order",
    "OrderExecutedInfo",
    "TradingPairProfile",
    "TradingPairId",
    "PriceFluctuation",
    "AddrStr",
    "AssetInfo",
    "AssetLedger",
    "AssetRestriction",
    "AssetRestrictions",
    "AssetType",
    "BTCAddress",
    "BTCHeader",
    "BTCHeaderInfo",
    "BTCNetwork",
    "BTCParams",
    "BTCTxInfo",
    "BondRequirement",
    "Chain",
    "ClaimRestriction",
    "Desc",
    "FixedAssetPower",
    "GlobalDistribution",
    "HandicapInfo",
    "Memo",
    "MinerLedger",
    "MiningDistribution",
    "NetworkType",
    "NominatorLedger",
    "NominatorProfile",
    "Order",
    "OrderExecutedInfo",
    "OrderId",
    "OrderInfo",
    "OrderType",
    "Price",
    "PriceFluctuation",
    "Selector",
    "Side",
    "SignedBalance",
    "StakingRequirement",
    "Token",
    "TradingHistoryIndex",
    "TradingPairId",
    "TradingPairInfo",
    "TradingPairProfile",
    "UnbondedIndex",
    "ValidatorLedger",
    "ValidatorProfile",
    "XRC20Selector",
]

base_ctags_cmd = [
    'ctags', '--format=2', '--excmd=pattern', '--fields=nksSaf', '--extras=+F',
    '--sort=no', '--append=no', '--extras=', '--language-force=rust',
    '--rust-kinds=cPstvfgieMnm', '--output-format=json', '--fields=-PF', '-f-'
]


#  Execute the system command and returns the lines of stdout.
def execute(cmd):
    result = subprocess.run(cmd, stdout=subprocess.PIPE)
    return result.stdout.decode('utf-8').split("\n")


#  Read the specific line of file, lnum is 1-based.
def read_line_at(fname, lnum):
    with open(fname, 'r') as reader:
        lines = reader.readlines()
        return lines[lnum - 1]


def read_struct_or_enum(fname, lnum):
    with open(fname, 'r') as reader:
        lines = reader.readlines()
        type_lines = []
        for i in range(lnum - 1, len(lines)):
            line = lines[i].strip()
            if not line:
                continue
            #  Skip the comment lines naively
            if line.startswith('//'):
                continue
            type_lines.append(line)
            # One line struct, e.g., pub struct Memo(Vec<u8>);
            if line.endswith(';'):
                return type_lines
            # Ignore the unrelated lines.
            if line.startswith('impl'):
                return type_lines
            #  If the starting line ends with {,
            #  stop at the first } then.
            if line.endswith('}'):
                #  print(type_lines)
                return type_lines


rs_files = execute(['fd', '-e', 'rs'])

enum_list = []
struct_list = []
typedef_list = []


#  Triage all the new types using ctags
def triage():
    for rs_file in rs_files:
        #  Skip the empty lines
        if not rs_file:
            continue

        cmd = base_ctags_cmd + [rs_file]

        for line in execute(cmd):
            if not line:
                continue

            tag_info = json.loads(line)

            tag_kind = tag_info['kind']
            tag_name = tag_info['name']

            if 'kind' in tag_info and 'scopeKind' not in tag_info:
                if tag_kind in TARGET_KINDS and tag_name in NEW_TYPES:
                    item = {'fname': rs_file, 'tag': tag_info}
                    if tag_kind == 'typedef':
                        typedef_list.append(item)
                    elif tag_kind == 'struct':
                        struct_list.append(item)
                    elif tag_kind == 'enum':
                        enum_list.append(item)


output = {}

suspicious = []


#  The parser may not work in such cases:
#  ..: 'Positive(T::Balance)',
#  ..: 'Negative(T::Balance)',
#  ..: 'Handicap<<T as Trait>::Price>',
def is_suspicious(s):
    return ':' in s or '<' in s


def parse_enum():
    for enum in enum_list:
        rs_file = enum['fname']
        tag_lnum = enum['tag']['line']
        key = enum['tag']['name']
        lines = read_struct_or_enum(rs_file, tag_lnum)
        enum['lines'] = lines
        fields = lines[1:-1]
        fields = list(map(lambda x: x.split(',')[0], fields))
        s = list(filter(is_suspicious, fields.copy()))
        suspicious.extend(s)
        output[key] = {"_enum": fields}


def parse_non_tuple_struct(lines, key):
    fields = lines[1:-1]
    fields_dict = {}
    for field in fields:
        var = ''
        ty = ''
        for item in field.split():
            if item.endswith(':'):
                var = item[:-1]
            if item.endswith(','):
                ty = item[:-1]
        fields_dict[var] = ty
    output[key] = fields_dict


def parse_tuple_struct(line, key):
    start = line.index('(')
    end = line.index(')')
    line = line[start + 1:end]
    inners = line.split()

    if len(inners) == 0:
        return

    if len(inners) == 1:
        inner = inners[0]
        ty = inner.rstrip(',')
        if ty in ALIAS:
            output[key] = ALIAS[ty]
        else:
            output[key] = ty
    else:
        value = []
        for inner in inners:
            ty = inner.rstrip(',')
            if ty in ALIAS:
                value.append(ALIAS[ty])
            else:
                value.append(ty)

        output[key] = value


def parse_struct():
    for struct in struct_list:
        rs_file = struct['fname']
        tag_lnum = struct['tag']['line']
        key = struct['tag']['name']
        lines = read_struct_or_enum(rs_file, tag_lnum)
        struct['lines'] = lines
        if len(lines) == 1:
            parse_tuple_struct(lines[0], key)
        if len(lines) > 1:
            parse_non_tuple_struct(lines, key)


def parse_typedef():
    for typedef in typedef_list:
        rs_file = typedef['fname']
        tag_lnum = typedef['tag']['line']
        key = typedef['tag']['name']
        line = read_line_at(rs_file, tag_lnum)
        line = line.strip()
        typedef['line'] = line
        #  Parse rule:
        #  1. split the line by '='
        #  2. find the item ending with ';'
        #  3. strip the last `;`
        items = line.split('=')
        filtered = list(filter(lambda x: x.endswith(';'), items))
        if len(filtered) > 0:
            #  = u32;
            #  = [u8; 4];
            value = filtered[0].strip()[:-1]
            if value in ALIAS:
                output[key] = ALIAS[value]
            else:
                if is_suspicious(value):
                    suspicious.append(value)
                output[key] = value


def write_json():
    os.chdir("./scripts")
    with open('chainx_types.json', 'w') as outfile:
        json.dump(output, outfile, indent=4, sort_keys=True)


def check_missing_types():
    pp = pprint.PrettyPrinter(indent=4)
    print('suspicious types:')
    pp.pprint(suspicious)
    print()
    missing = []
    for key in NEW_TYPES:
        if key not in output:
            missing.append(key)
    print('missing types:')
    pp.pprint(missing)


def main():
    triage()

    parse_enum()
    parse_struct()
    parse_typedef()

    write_json()

    check_missing_types()


main()
