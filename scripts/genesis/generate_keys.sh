#!/usr/bin/env bash

# Generate the various keys for the gensis.
#
# This script will print the info to stdout in a form which is slightly easier to be
# copied and pasted to the Rust source file.

set -e

if [[ -z "${SECRET}" ]]; then
  echo 'ERROR: $SECRET unset, please export the environment varible $SECRET first.'
  exit 1
fi

CHAIN=mainnet
DIR="keys"
CHAINX=../../target/release/chainx

print_validator_address() {
  address=$("$CHAINX" key inspect-key "$1" | tail -n 1 | awk '{print $NF}')
  echo "                // $address"
}

print_validator_id() {
  pubkey=$("$CHAINX" key inspect-key "$1" | tail -n 3 | head -1 | awk '{print $NF}')
  echo "                hex![\"${pubkey:2}\"].into(),"
}

print_address() {
  local_scheme=$1
  local_uri=$2
  address=$("$CHAINX" key inspect-key --scheme "$local_scheme" "$local_uri" | tail -n 1 | awk '{print $NF}')
  echo "            // $address"
}

print_account_key() {
  pubkey=$("$CHAINX" key inspect-key "$1" | tail -n 3 | head -1 | awk '{print $NF}')
  echo "            hex![\"${pubkey:2}\"].into(),"
}

print_aux_key() {
  local_scheme=$1
  local_uri=$2
  pubkey=$("$CHAINX" key inspect-key --scheme "$local_scheme"  "$local_uri" | tail -n 3 | head -1 | awk '{print $NF}')
  echo "            hex![\"${pubkey:2}\"].unchecked_into(),"
}

generate_aux_key() {
  key_type=$1
  scheme=$2
  dir=$3
  uri=$4
  "$CHAINX" key insert --chain=$CHAIN --key-type "$key_type" -d $dir --scheme "$scheme" --suri "$uri"
  print_address  "$scheme" "$uri"
  print_aux_key  "$scheme" "$uri"
}

main() {
  # Generate 5 pairs of genesis keys given the root secret
  for id in 1 2 3 4 5; do
    echo "SECRET//validator//$id:"
    echo "SECRET//blockauthor/$id, SECRET//babe//$id, SECRET//grandpa//$id, SECRET//im_online//$id, SECRET//authority_discovery//$id"
    echo

    echo "            ("
    print_validator_address "$SECRET//validator//$id"
    print_validator_id      "$SECRET//validator//$id"

    referral_id="Validator$id"
    echo "                b\""$referral_id"\".to_vec(),"
    echo "            ),"

    generate_aux_key babe sr25519 "$DIR/$id" "$SECRET//babe//$id"
    # Grandpa must use ed25519.
    generate_aux_key gran ed25519 "$DIR/$id" "$SECRET//grandpa//$id"
    generate_aux_key imon sr25519 "$DIR/$id" "$SECRET//im_online//$id"
    generate_aux_key audi sr25519 "$DIR/$id" "$SECRET//authority_discovery//$id"

    echo
  done

  echo 'Root:'
  print_address     sr25519 "$SECRET"
  print_account_key         "$SECRET"
  echo "The generated keys are in directory $DIR"
}

main
