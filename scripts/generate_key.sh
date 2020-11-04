#!/usr/bin/env bash

set -e

# SECRET="learn era fat agent beef tribe lens fame captain still soda owner"

if [[ -z "${SECRET}" ]]; then
  echo '$SECRET unset, please export the environment varible $SECRET first.'
  exit 1
fi

CHAIN=testnet

CHAINX=../target/release/chainx

print_validator_address() {
  address=$("$CHAINX" key inspect-key "$1" | tail -n 1 | awk '{print $NF}')
  echo "                // $address"
}

print_address() {
  address=$("$CHAINX" key inspect-key "$1" | tail -n 1 | awk '{print $NF}')
  echo "            // $address"
}

print_validator_id() {
  pubkey=$("$CHAINX" key inspect-key "$1" | tail -n 3 | head -1 | awk '{print $NF}')
  echo "                hex![\"${pubkey:2}\"].into(),"
}

print_account_key() {
  pubkey=$("$CHAINX" key inspect-key "$1" | tail -n 3 | head -1 | awk '{print $NF}')
  echo "            hex![\"${pubkey:2}\"].into(),"
}

print_other_key() {
  pubkey=$("$CHAINX" key inspect-key "$1" | tail -n 3 | head -1 | awk '{print $NF}')
  echo "            hex![\"${pubkey:2}\"].unchecked_into(),"
}

for id in 1 2 3; do

  dir="keys/$id"

  echo "          SECRET//validator//$id:"

  echo "SECRET//blockauthor/$id, SECRET//babe//$id, SECRET//grandpa//$id, SECRET//im_online//$id, SECRET//authority_discovery//$id"

  echo "            ("
  print_validator_address "$SECRET//validator//$id"
  print_validator_id  "$SECRET//validator//$id"

  echo "                b\"Validator"$id"\".to_vec(),"
  echo "            ),"

  print_address      "$SECRET//blockauthor//$id"
  print_account_key  "$SECRET//blockauthor//$id"

  "$CHAINX" key insert --chain=$CHAIN --key-type babe -d $dir --scheme sr25519 --suri "$SECRET//babe//$id"
  print_address   "$SECRET//babe//$id"
  print_other_key "$SECRET//babe//$id"

  "$CHAINX" key insert --chain=$CHAIN --key-type gran -d $dir --scheme ed25519 --suri "$SECRET//grandpa//$id"
  print_address    "$SECRET//grandpa//$id"
  print_other_key  "$SECRET//grandpa//$id"

  "$CHAINX" key insert --chain=$CHAIN --key-type imon -d $dir --scheme sr25519 --suri "$SECRET//im_online//$id"
  print_address    "$SECRET//im_online//$id"
  print_other_key  "$SECRET//im_online//$id"

  "$CHAINX" key insert --chain=$CHAIN --key-type audi -d $dir --scheme sr25519 --suri "$SECRET//authority_discovery//$id"
  print_address    "$SECRET//authority_discovery//$id"
  print_other_key  "$SECRET//authority_discovery//$id"
  echo
done


echo '                          Root:'
print_address "$SECRET"
print_account_key "$SECRET"
