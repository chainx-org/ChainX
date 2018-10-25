use rstd::prelude::*;
use bitcrypto::ripemd160;
use primitives::hash::H160;

#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum Opcode {
    // push value
    OP_0 = 0x00,
    OP_PUSHBYTES_20 = 0x14,
    OP_PUSHBYTES_33 = 0x21,
    OP_PUSHBYTES_65 = 0x41,
    OP_PUSHBYTES_71 = 0x47,
    OP_PUSHBYTES_72 = 0x48,
    OP_PUSHBYTES_73 = 0x49,
    OP_16 = 0x60,
    // control
    OP_RETURN = 0x6a,
    // stack ops
    OP_DUP = 0x76,
    // bit logic
    OP_EQUAL = 0x87,
    OP_EQUALVERIFY = 0x88,
    // crypto
    OP_HASH160 = 0xa9,
    OP_CHECKSIG = 0xac,
}


pub enum ParseScript {
    PubKey,
    PubKeyHash,
    ScriptHash,
    NullData,
    NotSupport,
}

pub fn parse_script(script: Vec<u8>) -> ParseScript {
    fn is_pay_to_public_key_hash(script: &Vec<u8>) -> bool {
        script.len() == 25 && script[0] == Opcode::OP_DUP as u8 &&
            script[1] == Opcode::OP_HASH160 as u8 &&
            script[2] == Opcode::OP_PUSHBYTES_20 as u8 &&
            script[23] == Opcode::OP_EQUALVERIFY as u8 &&
            script[24] == Opcode::OP_CHECKSIG as u8
    }

    fn is_pay_to_public_key(script: &Vec<u8>) -> bool {
        let len = match script[0] {
            x if x == Opcode::OP_PUSHBYTES_33 as u8 => 35,
            x if x == Opcode::OP_PUSHBYTES_65 as u8 => 67,
            _ => return false,
        };

        //OP_CHECKSIG
        script.len() == len && script[len - 1] == Opcode::OP_CHECKSIG as u8
    }

    fn is_pay_to_script_hash(script: &Vec<u8>) -> bool {
        script.len() == 23 && script[0] == Opcode::OP_HASH160 as u8 &&
            script[1] == Opcode::OP_PUSHBYTES_20 as u8 &&
            script[22] == Opcode::OP_EQUAL as u8
    }

    fn is_null_data_script(script: &Vec<u8>) -> bool {
        script[0] == Opcode::OP_RETURN as u8 &&
            {
                let mut pc = 1usize;
                while pc < script.len() {
                    let opcode = script[pc] as usize;

                    if opcode > Opcode::OP_16 as usize {
                        return false;
                    }

                    pc += opcode as usize;
                }
                true
            }
    }

    if is_pay_to_public_key_hash(&script) {
        return ParseScript::PubKeyHash;
    } else if is_pay_to_public_key(&script) {
        return ParseScript::PubKey;
    } else if is_pay_to_script_hash(&script) {
        return ParseScript::ScriptHash;
    } else if is_null_data_script(&script) {
        return ParseScript::NullData;
    } else {
        return ParseScript::NotSupport;
    }
}


pub fn parse_sigscript(script: Vec<u8>) -> Result<H160, ()> {
    let mut pc = 0usize;
    let mut data: Vec<Vec<u8>> = Vec::new();
    while pc < script.len() {
        if script[pc] == Opcode::OP_PUSHBYTES_33 as u8 ||
            script[pc] == Opcode::OP_PUSHBYTES_65 as u8 ||
            script[pc] == Opcode::OP_PUSHBYTES_71 as u8 ||
            script[pc] == Opcode::OP_PUSHBYTES_72 as u8 ||
            script[pc] == Opcode::OP_PUSHBYTES_73 as u8
            {

                let data_size = script[pc] as usize;
                pc += 1;
                data.push(Vec::from(&script[pc..pc + data_size]));
                pc += data_size;

            } else {
            return Err(());
        }
    }

    if let Some(pubkey) = data.last() {
        return Ok(ripemd160((*pubkey).as_slice()));
    }

    return Err(());
}