#!/usr/bin/env bash

set -e

SECRET="learn era fat agent beef tribe lens fame captain still soda owner"

CHAIN=testnet

CHAINX=../target/release/chainx

print_address() {
  echo "                       Address:" $("$CHAINX" key inspect-key "$1" | tail -n 1 | awk '{print $NF}')
}

print_pubkey() {
  echo "                    Public key:" $("$CHAINX" key inspect-key "$1" | tail -n 3 | head -1 | awk '{print $NF}')
}

for id in 1 2 3; do

  dir="keys/$id"

  echo "          SECRET//validator//$id:"
  print_address "$SECRET//validator//$id"
  print_pubkey  "$SECRET//validator//$id"

  echo "         SECRET//blockauthor/$id:"
  print_address "$SECRET//blockauthor//$id"
  print_pubkey  "$SECRET//blockauthor//$id"

  "$CHAINX" key insert --chain=$CHAIN --key-type babe -d $dir --scheme sr25519 --suri "$SECRET//$id//babe"
  echo "               SECRET//babe//$id:"
  print_address "$SECRET//babe//$id"
  print_pubkey  "$SECRET//babe//$id"

  "$CHAINX" key insert --chain=$CHAIN --key-type gran -d $dir --scheme ed25519 --suri "$SECRET//$id//grandpa"
  echo "            SECRET//grandpa//$id:"
  print_address "$SECRET//grandpa//$id"
  print_pubkey  "$SECRET//grandpa//$id"

  "$CHAINX" key insert --chain=$CHAIN --key-type imon -d $dir --scheme sr25519 --suri "$SECRET//$id//im_online"
  echo "          SECRET//im_online//$id:"
  print_address "$SECRET//im_online//$id"
  print_pubkey  "$SECRET//im_online//$id"

  "$CHAINX" key insert --chain=$CHAIN --key-type audi -d $dir --scheme sr25519 --suri "$SECRET//$id//authority_discovery"
  echo "SECRET//authority_discovery//$id:"
  print_address "$SECRET//authority_discovery//$id"
  print_pubkey  "$SECRET//authority_discovery//$id"
  echo
done


echo '                          Root:'
print_pubkey "$SECRET"
print_address "$SECRET"
