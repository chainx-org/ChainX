const { encodeAddress, blake2AsHex } = require('@polkadot/util-crypto');
const { Buffer } = require('buffer');
const { program } = require('commander');

function validateH160(h160Addr) {
    const re = /0x[0-9A-Fa-f]{40}/g;
    if(!re.test(h160Addr)) {
        throw 'Invalid EVM(H160) address provided!';
    }
}

function main(h160Addr) {
    validateH160(h160Addr);
    const addressBytes = Buffer.from(h160Addr.slice(2), 'hex');
    const prefixBytes = Buffer.from('evm:');
    const convertBytes = Uint8Array.from(Buffer.concat([ prefixBytes, addressBytes ]));
    const finalAddressHex = blake2AsHex(convertBytes, 256);

    console.log("evm    address: ", h160Addr);
    console.log("chainx account: ", encodeAddress(finalAddressHex, 44));
}

program
    .argument('<evm-address>', 'The evm address.')
    .action(main)
    .parse(process.argv);