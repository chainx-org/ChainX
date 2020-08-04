#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json
import os
import pprint
import re
import shutil
import subprocess

#  You need to install ctags and fd to run this script.
if not shutil.which('ctags'):
    print(
        'Please install https://github.com/universal-ctags/ctags to continue')
    os._exit(1)

if not shutil.which('fd'):
    print('Please install https://github.com/sharkdp/fd to continue')
    os._exit(1)

NEW_TYPES = [
    "AssetType",
    "SignedBalance",
    "Chain",
    "AddrStr",
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
    "BtcAddress",
    "BtcHeader",
    "BtcHeaderInfo",
    "BtcNetwork",
    "BtcParams",
    "BondRequirement",
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
    "RpcNominatorLedger",
]

# Change the working directory to project root directory.
os.chdir("../..")

TARGET_KINDS = ['typedef', 'enum', 'struct']

ALIAS = {'Vec<u8>': 'Text'}

BASE_CTAGS_CMD = [
    'ctags', '--format=2', '--excmd=pattern', '--fields=nksSaf', '--extras=+F',
    '--sort=no', '--append=no', '--extras=', '--language-force=rust',
    '--rust-kinds=cPstvfgieMnm', '--output-format=json', '--fields=-PF', '-f-'
]


#  total_balance => totalBalance
def snake_to_camel(word):
    s = ''.join(x.capitalize() or '_' for x in word.split('_'))
    # Lowercase first character of String
    return s[0].lower() + s[1:]


CHAINX_RPC_MANUAL_JSON = './scripts/chainx-js/chainx_rpc_manual.json'
CHAINX_TYPES_MANUAL_JSON = './scripts/chainx-js/chainx_types_manual.json'

MANUAL = {}
# Read the types crated manually and convert the under_score_case to camelCase.
with open(CHAINX_TYPES_MANUAL_JSON) as json_file:
    raw_manual = json.load(json_file)
    for k1, v1 in raw_manual.items():
        if isinstance(v1, dict) and '_enum' not in v1:
            vv = {}
            for k2, v2 in v1.items():
                kk = snake_to_camel(k2)
                vv[kk] = v2
            MANUAL[k1] = vv
        else:
            MANUAL[k1] = v1


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
                return type_lines


WELL_KNOWN_TYPES = ['AccountId', 'Balance', 'BlockNumber']

# Find all *.rs in ChainX project, following the gitignore rule.
rs_files = execute(['fd', '-e', 'rs'])

enum_list = []
struct_list = []
typedef_list = []

output = {}

# The generated content for this item might be problematic.
suspicious = []

maybe_new_types = {}

# New types discovered positively in the processing, a subset of
# maybe_new_types.
positive_new_types = []


#  Triage all the new types using ctags
def triage():
    for rs_file in rs_files:
        #  Skip the empty lines
        if not rs_file:
            continue

        cmd = BASE_CTAGS_CMD + [rs_file]

        for line in execute(cmd):
            if not line:
                continue

            tag_info = json.loads(line)

            tag_kind = tag_info['kind']
            tag_name = tag_info['name']

            if 'kind' in tag_info and 'scopeKind' not in tag_info:
                #  Some new types exists in the fields
                if tag_kind in TARGET_KINDS and tag_name not in NEW_TYPES:
                    if tag_kind in ('typedef', 'struct', 'enum'):
                        # Ignore types in irrelevant files
                        if ('mock' not in rs_file
                                and tag_name not in WELL_KNOWN_TYPES):
                            maybe_new_types[tag_name] = {
                                'fname': rs_file,
                                'tag': tag_info
                            }

                #  Explicit new types
                if tag_kind in TARGET_KINDS and tag_name in NEW_TYPES:
                    item = {'fname': rs_file, 'tag': tag_info}
                    if tag_kind == 'typedef':
                        typedef_list.append(item)
                    elif tag_kind == 'struct':
                        struct_list.append(item)
                    elif tag_kind == 'enum':
                        enum_list.append(item)


#  The parser may not work in such cases:
#  ..: 'Positive(T::Balance)',
#  ..: 'Negative(T::Balance)',
#  ..: 'Handicap<<T as Trait>::Price>',
def is_suspicious(s):
    return ':' in s or '<' in s


def parse_enum_impl(enum):
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
        if field.strip().startswith('#[cfg'):
            continue
        var = ''
        ty = ''
        for item in field.split():
            if item.endswith(':'):
                var = snake_to_camel(item[:-1])
            if item.endswith(','):
                ty = item[:-1]
                # Try finding the nested structs/enums/typedefs
                if (ty in maybe_new_types
                        and maybe_new_types[ty] not in positive_new_types):
                    positive_new_types.append(maybe_new_types[ty])
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


def parse_struct_impl(struct):
    rs_file = struct['fname']
    tag_lnum = struct['tag']['line']
    key = struct['tag']['name']
    lines = read_struct_or_enum(rs_file, tag_lnum)
    struct['lines'] = lines
    if len(lines) == 1:
        parse_tuple_struct(lines[0], key)
    if len(lines) > 1:
        parse_non_tuple_struct(lines, key)


def parse_typedef_impl(typedef):
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


def parse_enum():
    for enum in enum_list:
        parse_enum_impl(enum)


def parse_struct():
    for struct in struct_list:
        parse_struct_impl(struct)


def parse_typedef():
    for typedef in typedef_list:
        parse_typedef_impl(typedef)


def check_missing_types():
    pp = pprint.PrettyPrinter(indent=4)
    print('These types might be problematic:')
    pp.pprint(suspicious)
    print()
    missing = []
    for key in NEW_TYPES:
        if key not in output:
            #  Inject the hard coded type
            if key in MANUAL:
                output[key] = MANUAL[key]
            else:
                missing.append(key)
    print('These types are still missing:')
    pp.pprint(missing)


def parse_nested_elements():
    for new_type in positive_new_types:
        kind = new_type['tag']['kind']
        if kind == 'typedef':
            parse_typedef_impl(new_type)
        elif kind == 'struct':
            parse_struct_impl(new_type)
        elif kind == 'enum':
            parse_enum_impl(new_type)
        else:
            pass


#  typdef, enum, struct
def build_types():
    triage()

    parse_enum()
    parse_struct()
    parse_typedef()

    parse_nested_elements()

    check_missing_types()


rpc_dict = {}


def parse_rpc_params(fn):
    params = []
    for item in fn.split(','):
        if item.endswith('self'):
            continue
        if ':' in item:
            [name, ty] = item.split(':')
            name = name.strip()
            ty = ty.strip()
            #  Special case
            if ty == 'Option<BlockHash>':
                params.append({
                    'name': name,
                    'type': 'Hash',
                    'isOptional': True
                })
            else:
                params.append({'name': name, 'type': ty})

    return params


def parse_rpc_api(xmodule, description, inner_fn, line_fn):
    [fn, result] = line_fn.split('->')

    if xmodule not in rpc_dict:
        rpc_dict[xmodule] = {}

    params = parse_rpc_params(fn)

    #  Result<BTreeMap<AssetId, TotalAssetInfo>>;
    # len('Result<') = 7
    # >; = 2
    ok_result = result[8:-2]
    rpc_dict[xmodule][inner_fn] = {
        'description': description,
        'params': params,
        'type': ok_result
    }


def build_rpc():
    #  Assume all the API definition is in foo/rpc/src/lib.rs
    rpc_rs_files = list(filter(lambda x: '/rpc/src/lib.rs' in x, rs_files))

    for fname in rpc_rs_files:
        with open(fname, 'r') as reader:
            lines = reader.readlines()
            idx = 0
            for line in lines:
                idx += 1
                if '[rpc(name =' in line:
                    if lines[idx - 2].lstrip().startswith('///'):
                        comment = lines[idx - 2].strip()
                        description = comment[3:].strip()
                    else:
                        description = 'Some description'
                    #  [rpc(name = "xassets_getAssets")] --> xassets_getAssets
                    matches = re.findall(r'\"(.+?)\"', line)
                    name = matches[0]
                    [xmodule, inner_fn] = name.split('_')

                    #  Only handle the ChainX specific RPC, starting with x
                    if xmodule.startswith('x'):
                        fn_lines = []
                        #  Normally the fn defintion won't more than 10 lines
                        for i in range(idx, idx + 10):
                            fn_lines.append(lines[i].strip())
                            if lines[i].strip().endswith(';'):
                                break
                        line_fn = ''.join(fn_lines)
                        parse_rpc_api(xmodule, description, inner_fn, line_fn)


def write_json(output_json, output_fname):
    with open(output_fname, 'w') as outfile:
        #  NOTE: Do not enable sort_keys as the fields are order sensitive
        #  regarding the encode/decode.
        json.dump(output_json, outfile, indent=4, sort_keys=False)


def write_types_and_rpc():
    for k in MANUAL:
        #  Always override with types created manually.
        output[k] = MANUAL[k]

    with open(CHAINX_RPC_MANUAL_JSON) as json_file:
        RPC_MANUAL = json.load(json_file)

    os.chdir("./scripts/chainx-js")
    write_json(output, 'res/chainx_types.json')

    for xmodule, fns in rpc_dict.items():
        if xmodule in RPC_MANUAL:
            manual_fns = RPC_MANUAL[xmodule]
            for k in fns:
                if k in manual_fns:
                    fns[k] = manual_fns[k]

    rpc_output = {}
    #  Inject rpc decoration
    rpc_output['rpc'] = rpc_dict
    write_json(rpc_output, 'res/chainx_rpc.json')


def main():
    build_types()
    build_rpc()
    write_types_and_rpc()


main()
