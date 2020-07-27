#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import argparse
import http.client
import json
import mimetypes
import os

conn = http.client.HTTPSConnection("blockstream.info")

NETWORK_TYPE = 'Mainnet'


def get_from_blockstream_api(endpoint):
    payload = ''
    headers = {'Content-Type': 'application/json'}

    if NETWORK_TYPE == 'Testnet':
        conn.request("GET", '/testnet' + endpoint, payload, headers)
    else:
        conn.request("GET", endpoint, payload, headers)

    res = conn.getresponse()
    data = res.read()
    return data.decode('utf-8')


def get_block(block_hash):
    endpoint = '/api/block/' + block_hash
    return get_from_blockstream_api(endpoint)


def get_block_hash(block_height):
    endpoint = '/api/block-height/' + block_height
    return get_from_blockstream_api(endpoint)


def main():
    parser = argparse.ArgumentParser(
        description='Generate ChainX Bitcoin Genesis Block Header.')
    parser.add_argument(
        'height',
        type=str,
        help='block height for the Bitcoin genesis block header')
    parser.add_argument('--network',
                        nargs='?',
                        default='Mainnet',
                        help='connect to Bitcoin testnet instead of mainnet')
    args = parser.parse_args()

    if args.network != 'Mainnet':
        NETWORK_TYPE = 'Testnet'

    print('Generating ' + args.network + ' Bitcoin Block Header for #' +
          args.height + ':\n')

    blk_hash = get_block_hash(args.height)
    print('hash: ' + blk_hash + '\n')
    full_header = json.loads(get_block(blk_hash))
    generated = {
        'version': full_header['version'],
        'previous_header_hash': full_header['previousblockhash'],
        'merkle_root_hash': full_header['merkle_root'],
        'time': full_header['timestamp'],
        'bits': full_header['bits'],
        'nonce': full_header['nonce'],
        'height': args.height,
        'hash': blk_hash
    }
    print('Generated Block header info:\n')
    print(json.dumps(generated, indent=4))

    #  Switch the working directory to project root directory.
    cur_file = os.path.abspath(__file__)
    genesis_dir = os.path.dirname(cur_file)
    scripts_dir = os.path.dirname(genesis_dir)
    chainx_dir = os.path.dirname(scripts_dir)
    os.chdir(chainx_dir)

    output_fname = 'cli/src/res/btc_genesis_header_' + args.network.lower(
    ) + '.json'
    with open(output_fname, 'w') as outfile:
        json.dump(generated, outfile, indent=4, sort_keys=False)

    print()
    print(output_fname + ' has been updated')


main()
