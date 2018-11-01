//! Serialized script, used inside transaction inputs and outputs.

#[cfg(feature = "std")]
use std::fmt;
use rstd::ops;
use super::bytes::Bytes;
use super::keys::{AddressHash, Public};
use super::{Opcode, Error};
use super::keys;

/// Maximum number of public keys per multisig
pub const MAX_PUBKEYS_PER_MULTISIG: usize = 20;

/// Classified script type
#[derive(PartialEq, Debug)]
pub enum ScriptType {
	NonStandard,
	PubKey,
	PubKeyHash,
	ScriptHash,
	Multisig,
	NullData,
	WitnessScript,
	WitnessKey,
}

/// Address from Script
#[derive(PartialEq, Debug)]
pub struct ScriptAddress {
	/// The type of the address.
	pub kind: keys::Type,
	/// Public key hash.
	pub hash: AddressHash,
}

impl<'a> From<&'a keys::Address> for ScriptAddress {
    fn from(address: &'a keys::Address) -> Self {
        match address.kind {
            keys::Type::P2PKH => ScriptAddress::new_p2pkh(address.hash.clone()),
            keys::Type::P2SH => ScriptAddress::new_p2sh(address.hash.clone()),
        }
    }
}

impl ScriptAddress {
	/// Creates P2PKH-type ScriptAddress
	pub fn new_p2pkh(hash: AddressHash) -> Self {
		ScriptAddress {
			kind: keys::Type::P2PKH,
			hash: hash,
		}
	}

	/// Creates P2SH-type ScriptAddress
	pub fn new_p2sh(hash: AddressHash) -> Self {
		ScriptAddress {
			kind: keys::Type::P2SH,
			hash: hash,
		}
	}
}

/// Serialized script, used inside transaction inputs and outputs.
#[derive(PartialEq, Debug)]
pub struct Script {
	data: Bytes,
}

impl From<&'static str> for Script {
	fn from(s: &'static str) -> Self {
		Script::new(s.into())
	}
}

impl From<Bytes> for Script {
	fn from(s: Bytes) -> Self {
		Script::new(s)
	}
}

impl From<Vec<u8>> for Script {
	fn from(v: Vec<u8>) -> Self {
		Script::new(v.into())
	}
}

impl From<Script> for Bytes {
	fn from(script: Script) -> Self {
		script.data
	}
}

impl Script {
	/// Script constructor.
	pub fn new(data: Bytes) -> Self {
		Script {
			data: data,
		}
	}

	pub fn to_bytes(&self) -> Bytes {
		self.data.clone()
	}

	/// Is empty script
	pub fn is_empty(&self) -> bool {
		self.data.len() == 0
	}

	/// Extra-fast test for pay-to-public-key-hash (P2PKH) scripts.
	pub fn is_pay_to_public_key_hash(&self) -> bool {
		self.data.len() == 25 &&
			self.data[0] == Opcode::OP_DUP as u8 &&
			self.data[1] == Opcode::OP_HASH160 as u8 &&
			self.data[2] == Opcode::OP_PUSHBYTES_20 as u8 &&
			self.data[23] == Opcode::OP_EQUALVERIFY as u8 &&
			self.data[24] == Opcode::OP_CHECKSIG as u8
	}

	/// Extra-fast test for pay-to-public-key (P2PK) scripts.
	pub fn is_pay_to_public_key(&self) -> bool {
		if self.data.is_empty() {
			return false;
		}

		let len = match self.data[0] {
			x if x == Opcode::OP_PUSHBYTES_33 as u8 => 35,
			x if x == Opcode::OP_PUSHBYTES_65 as u8 => 67,
			_ => return false,
		};

		self.data.len() == len && self.data[len - 1] == Opcode::OP_CHECKSIG as u8
	}

	/// Extra-fast test for pay-to-script-hash (P2SH) scripts.
	pub fn is_pay_to_script_hash(&self) -> bool {
		self.data.len() == 23 &&
			self.data[0] == Opcode::OP_HASH160 as u8 &&
			self.data[1] == Opcode::OP_PUSHBYTES_20 as u8 &&
			self.data[22] == Opcode::OP_EQUAL as u8
	}

	/// Extra-fast test for pay-to-witness-key-hash scripts.
	pub fn is_pay_to_witness_key_hash(&self) -> bool {
		self.data.len() == 22 &&
			self.data[0] == Opcode::OP_0 as u8 &&
			self.data[1] == Opcode::OP_PUSHBYTES_20 as u8
	}

	/// Parse witness program. Returns Some(witness program version, code) or None if not a witness program.
	pub fn parse_witness_program(&self) -> Option<(u8, &[u8])> {
		if self.data.len() < 4 || self.data.len() > 42 || self.data.len() != self.data[1] as usize + 2 {
			return None;
		}
		let witness_version = match Opcode::from_u8(self.data[0]) {
			Some(Opcode::OP_0) => 0,
			Some(x) if x >= Opcode::OP_1 && x <= Opcode::OP_16 => (x as u8) - (Opcode::OP_1 as u8) + 1,
			_ => return None,
		};
		let witness_program = &self.data[2..];
		Some((witness_version, witness_program))
	}

	/// Extra-fast test for pay-to-witness-script-hash scripts.
	pub fn is_pay_to_witness_script_hash(&self) -> bool {
		self.data.len() == 34 &&
			self.data[0] == Opcode::OP_0 as u8 &&
			self.data[1] == Opcode::OP_PUSHBYTES_32 as u8
	}

	/// Extra-fast test for multisig scripts.
	pub fn is_multisig_script(&self) -> bool {
		if self.data.len() < 3 {
			return false;
		}

		let siglen = match self.get_opcode(0) {
			Ok(Opcode::OP_0) => 0,
			Ok(o) if o >= Opcode::OP_1 && o <= Opcode::OP_16 => o as u8 - (Opcode::OP_1 as u8 - 1),
			_ => return false,
		};

		let keylen = match self.get_opcode(self.data.len() - 2) {
			Ok(Opcode::OP_0) => 0,
			Ok(o) if o >= Opcode::OP_1 && o <= Opcode::OP_16 => o as u8 - (Opcode::OP_1 as u8 - 1),
			_ => return false,
		};

		if siglen > keylen {
			return false;
		}

		if self.data[self.data.len() - 1] != Opcode::OP_CHECKMULTISIG as u8 {
			return false;
		}

		let mut pc = 1;
		let mut keys = 0;
		while pc < self.len() - 2 {
			let instruction = match self.get_instruction(pc) {
				Ok(i) => i,
				_ => return false,
			};

			match instruction.opcode {
				Opcode::OP_PUSHBYTES_33 |
				Opcode::OP_PUSHBYTES_65 => keys += 1,
				_ => return false,
			}

			pc += instruction.step;
		}

		keys == keylen
	}

	pub fn is_null_data_script(&self) -> bool {
		// TODO: optimise it
		!self.data.is_empty() &&
			self.data[0] == Opcode::OP_RETURN as u8 &&
			self.subscript(1).is_push_only()
	}

	pub fn subscript(&self, from: usize) -> Script {
		self.data[from..].to_vec().into()
	}

	pub fn find_and_delete(&self, data: &[u8]) -> Script {
		let mut result = Vec::new();
		let mut current = 0;
		let len = data.len();
		let end = self.data.len();

		if len > end || len == 0 {
			return self.data.to_vec().into()
		}

		while current < end - len {
			if &self.data[current..current + len] != data {
				result.push(self.data[current]);
				current += 1;
			} else {
				current += len;
			}
		}

		result.extend_from_slice(&self.data[current..]);
		result.into()
	}

    pub fn extract_rear(&self, key: char) -> Vec<u8> {
        let key = key as u8;
        let mut result = Vec::new();
        let end = self.data.len();
        let mut current = 0;
        while current < end {
            if self.data[current] == key {
                break;
            }
            current += 1;
        }
        result.extend_from_slice(&self.data[current + 1..]);
        result
    }
 
	pub fn get_opcode(&self, position: usize) -> Result<Opcode, Error> {
		Opcode::from_u8(self.data[position]).ok_or(Error::BadOpcode)
	}

	pub fn get_instruction(&self, position: usize) -> Result<Instruction, Error> {
		let opcode = self.get_opcode(position)?;
		let instruction = match opcode {
			Opcode::OP_PUSHDATA1 |
			Opcode::OP_PUSHDATA2 |
			Opcode::OP_PUSHDATA4 => {
				let len = match opcode {
					Opcode::OP_PUSHDATA1 => 1,
					Opcode::OP_PUSHDATA2 => 2,
					_ => 4,
				};

				let slice = self.take(position + 1, len)?;
				let n = read_usize(slice, len)?;
				let bytes = self.take(position + 1 + len, n)?;
				Instruction {
					opcode: opcode,
					step: len + n + 1,
					data: Some(bytes),
				}
			},
			o if o <= Opcode::OP_PUSHBYTES_75 => {
				let bytes = self.take(position + 1, opcode as usize)?;
				Instruction {
					opcode: o,
					step: opcode as usize + 1,
					data: Some(bytes),
				}
			},
			_ => Instruction {
				opcode: opcode,
				step: 1,
				data: None,
			}
		};

		Ok(instruction)
	}

	#[inline]
	pub fn take(&self, offset: usize, len: usize) -> Result<&[u8], Error> {
		if offset + len > self.data.len() {
			Err(Error::BadOpcode)
		} else {
			Ok(&self.data[offset..offset + len])
		}
	}

	/// Returns Script without OP_CODESEPARATOR opcodes
	pub fn without_separators(&self) -> Script {
		let mut pc = 0;
		let mut result = Vec::new();

		while pc < self.len() {
			match self.get_instruction(pc) {
				Ok(instruction) => {
					if instruction.opcode != Opcode::OP_CODESEPARATOR {
						result.extend_from_slice(&self[pc..pc + instruction.step]);
					}

					pc += instruction.step;
				},
				_ => {
					result.push(self[pc]);
					pc += 1;
				}
			}
		}

		result.into()
	}

	/// Returns true if script contains only push opcodes
	pub fn is_push_only(&self) -> bool {
		let mut pc = 0;
		while pc < self.len() {
			let instruction = match self.get_instruction(pc) {
				Ok(i) => i,
				_ => return false,
			};

			if instruction.opcode > Opcode::OP_16 {
				return false;
			}

			pc += instruction.step;
		}
		true
	}

	pub fn script_type(&self) -> ScriptType {
		if self.is_pay_to_public_key() {
			ScriptType::PubKey
		} else if self.is_pay_to_public_key_hash() {
			ScriptType::PubKeyHash
		} else if self.is_pay_to_script_hash() {
			ScriptType::ScriptHash
		} else if self.is_multisig_script() {
			ScriptType::Multisig
		} else if self.is_null_data_script() {
			ScriptType::NullData
		} else if self.is_pay_to_witness_key_hash() {
			ScriptType::WitnessKey
		} else if self.is_pay_to_witness_script_hash() {
			ScriptType::WitnessScript
		} else {
			ScriptType::NonStandard
		}
	}

	pub fn iter(&self) -> Instructions {
		Instructions { position: 0, script: self }
	}

	pub fn opcodes(&self) -> Opcodes {
		Opcodes { position: 0, script: self }
	}

	pub fn sigops_count(&self, serialized_script: bool) -> usize {
		let mut last_opcode = Opcode::OP_0;
		let mut total = 0;
		for opcode in self.opcodes() {
			let opcode = match opcode {
				Ok(opcode) => opcode,
				// If we push an invalid element, all previous CHECKSIGs are counted
				_ => return total,
			};

			match opcode {
				Opcode::OP_CHECKSIG | Opcode::OP_CHECKSIGVERIFY => {
					total += 1;
				},
				Opcode::OP_CHECKMULTISIG | Opcode::OP_CHECKMULTISIGVERIFY => {
					if serialized_script && last_opcode.is_within_op_n() {
						total += last_opcode.decode_op_n() as usize;
					} else {
						total += MAX_PUBKEYS_PER_MULTISIG;
					}
				},
				_ => (),
			};

			last_opcode = opcode;
		}

		total
	}

	pub fn num_signatures_required(&self) -> u8 {
		if self.is_multisig_script() {
			return match self.data[0] {
				x if x == Opcode::OP_0 as u8 => 0,
				x => x - (Opcode::OP_1 as u8) + 1,
			};
		}
		return 1;
	}

    pub fn extract_multi_scriptsig(&self) -> Result<(Vec<Bytes>, Script), keys::Error> { //[sig], redeem
		let mut pc = 1;
        let mut vec: Vec<Bytes> = Vec::new();
		while pc < self.len() - 2 {
			let instruction = self.get_instruction(pc).expect("this method depends on previous check in script_type()");
			let data = instruction.data.expect("this method depends on previous check in script_type()");
            vec.push(data.into()); 
			pc += instruction.step;
		}
        if let Some(script) = vec.pop() {
            return Ok((vec, script.into()));
        }
        return Err(keys::Error::InvalidSignature);
    }

	pub fn extract_destinations(&self) -> Result<Vec<ScriptAddress>, keys::Error> {
		match self.script_type() {
			ScriptType::NonStandard => {
				Ok(vec![])
			},
			ScriptType::PubKey => {
				Public::from_slice(match self.data[0] {
					x if x == Opcode::OP_PUSHBYTES_33 as u8 => &self.data[1..34],
					x if x == Opcode::OP_PUSHBYTES_65 as u8 => &self.data[1..66],
					_ => unreachable!(), // because we are relying on script_type() checks here
				})
				.map(|public| vec![ScriptAddress::new_p2pkh(public.address_hash())])
			},
			ScriptType::PubKeyHash => {
				Ok(vec![
					ScriptAddress::new_p2pkh(self.data[3..23].into()),
				])
			},
			ScriptType::ScriptHash => {
				Ok(vec![
					ScriptAddress::new_p2sh(self.data[2..22].into()),
				])
			},
			ScriptType::Multisig => {
				let mut addresses: Vec<ScriptAddress> = Vec::new();
				let mut pc = 1;
				while pc < self.len() - 2 {
					let instruction = self.get_instruction(pc).expect("this method depends on previous check in script_type()");
					let data = instruction.data.expect("this method depends on previous check in script_type()");
					let address = Public::from_slice(data)?.address_hash();
					addresses.push(ScriptAddress::new_p2pkh(address));
					pc += instruction.step;
				}
				Ok(addresses)
			},
			ScriptType::NullData => {
				Ok(vec![])
			},
			ScriptType::WitnessScript => {
				Ok(vec![]) // TODO
			},
			ScriptType::WitnessKey => {
				Ok(vec![]) // TODO
			},
		}
	}

	pub fn pay_to_script_hash_sigops(&self, prev_out: &Script) -> usize {
		if !prev_out.is_pay_to_script_hash() {
			return 0;
		}

		if self.data.is_empty() || !self.is_push_only() {
			return 0;
		}

		let script: Script = self.iter().last()
			.expect("self.data.is_empty() == false; qed")
			.expect("self.data.is_push_only()")
			.data.expect("self.data.is_push_only()")
			.to_vec()
			.into();

		script.sigops_count(true)
	}
}

pub struct Instructions<'a> {
	position: usize,
	script: &'a Script,
}

pub struct Opcodes<'a> {
	position: usize,
	script: &'a Script,
}

impl<'a> Iterator for Instructions<'a> {
	type Item = Result<Instruction<'a>, Error>;

	fn next(&mut self) -> Option<Result<Instruction<'a>, Error>> {

		if self.script.len() <= self.position { return None; }

		let instruction = match self.script.get_instruction(self.position) {
			Ok(x) => x,
			Err(e) => return Some(Err(e)),
		};

		self.position += instruction.step;

		Some(Ok(instruction))
	}
}


impl<'a> Iterator for Opcodes<'a> {
	type Item = Result<Opcode, Error>;

	fn next(&mut self) -> Option<Result<Opcode, Error>> {

		if self.script.len() <= self.position { return None; }

		let instruction = match self.script.get_instruction(self.position) {
			Ok(x) => x,
			Err(e) => return Some(Err(e)),
		};

		self.position += instruction.step;

		Some(Ok(instruction.opcode))
	}
}

impl ops::Deref for Script {
	type Target = [u8];

	fn deref(&self) -> &Self::Target {
		&self.data
	}
}

pub struct Instruction<'a> {
	pub opcode: Opcode,
	pub step: usize,
	pub data: Option<&'a [u8]>,
}

fn read_usize(data: &[u8], size: usize) -> Result<usize, Error> {
	if data.len() < size {
		return Err(Error::BadOpcode);
	}

	let result = data
		.iter()
		.take(size)
		.enumerate()
		.fold(0, |acc, (i, x)| acc + ((*x as usize) << (i * 8)));
	Ok(result)
}

#[cfg(feature = "std")]
impl fmt::Display for Script {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut pc = 0;

		while pc < self.len() {
			let instruction = match self.get_instruction(pc) {
				Ok(i) => i,
				Err(e) => return e.fmt(f),
			};

			match instruction.data {
				Some(data) => writeln!(f, "{:?} 0x{:?}", instruction.opcode, Bytes::from(data.to_vec()))?,
				None => writeln!(f, "{:?}", instruction.opcode)?,
			}

			pc += instruction.step;
		}

		Ok(())
	}
}

pub type ScriptWitness = Vec<Bytes>;

/// Passed bytes array is a commitment script?
/// https://github.com/bitcoin/bips/blob/master/bip-0141.mediawiki#Commitment_structure
pub fn is_witness_commitment_script(script: &[u8]) -> bool {
	script.len() >= 38 &&
		script[0] == Opcode::OP_RETURN as u8 &&
		script[1] == 0x24 &&
		script[2] == 0xAA &&
		script[3] == 0x21 &&
		script[4] == 0xA9 &&
		script[5] == 0xED
}

#[cfg(test)]
mod tests {
    use super::*;
    const redeem: &'static str = "52210257aff1270e3163aaae9d972b3d09a2385e0d4877501dbeca3ee045f8de00d21c2103fd58c689594b87bbe20a9a00091d074dc0d9f49a988a7ad4c2575adeda1b507c2102bb2a5aa53ba7c0d77bdd86bb9553f77dd0971d3a6bb6ad609787aa76eb17b6b653ae";
    const scriptsig1: &'static str = "00483045022100c0076941e39126f1bd0102d6df278470802ca8b694f8e39467121dc9ecc4d46802204ab7e3128bd0a93a30d1d5ea4db57cc8ba2d4c39172c2d2e536787e0b152bffe014c6952210257aff1270e3163aaae9d972b3d09a2385e0d4877501dbeca3ee045f8de00d21c2103fd58c689594b87bbe20a9a00091d074dc0d9f49a988a7ad4c2575adeda1b507c2102bb2a5aa53ba7c0d77bdd86bb9553f77dd0971d3a6bb6ad609787aa76eb17b6b653ae";
    const scriptsig2: &'static str = "00483045022100c0076941e39126f1bd0102d6df278470802ca8b694f8e39467121dc9ecc4d46802204ab7e3128bd0a93a30d1d5ea4db57cc8ba2d4c39172c2d2e536787e0b152bffe014730440220731394ffbf7d068393a2b6146e09f16bd9e39c16d04f38461a4c6991a725609202202633acd7cbf14883736f8e6376aa9090d0adacf73bc76ff5f95dca069caad593014c6952210257aff1270e3163aaae9d972b3d09a2385e0d4877501dbeca3ee045f8de00d21c2103fd58c689594b87bbe20a9a00091d074dc0d9f49a988a7ad4c2575adeda1b507c2102bb2a5aa53ba7c0d77bdd86bb9553f77dd0971d3a6bb6ad609787aa76eb17b6b653ae";
    const sig1: &'static str = "3045022100c0076941e39126f1bd0102d6df278470802ca8b694f8e39467121dc9ecc4d46802204ab7e3128bd0a93a30d1d5ea4db57cc8ba2d4c39172c2d2e536787e0b152bffe01";
    const sig2: &'static str = "30440220731394ffbf7d068393a2b6146e09f16bd9e39c16d04f38461a4c6991a725609202202633acd7cbf14883736f8e6376aa9090d0adacf73bc76ff5f95dca069caad59301";
    #[test]
    fn redeem_script() {
        let script: Script = redeem.into();
        assert_eq!(script.is_multisig_script(), true); 
    }

    #[test]
    fn input_1() {
       let script: Script = scriptsig1.into();
       let (sigs, dem) = script.extract_multi_scriptsig().unwrap(); 
       let sig_bytes: Bytes = sig1.into();
       assert_eq!(sig_bytes, sigs[0]);
       let script: Script = redeem.into();
       assert_eq!(script, dem);
       assert_eq!(sigs.len(), 1);
    }

    #[test]
    fn input_2() {
       let script: Script = scriptsig2.into();
       let (sigs, dem) = script.extract_multi_scriptsig().unwrap(); 
       let sig_bytes1: Bytes = sig1.into();
       assert_eq!(sig_bytes1, sigs[0]);
       let sig_bytes2: Bytes = sig2.into();
       assert_eq!(sig_bytes2, sigs[1]);
       let script: Script = redeem.into();
       assert_eq!(script, dem);
       assert_eq!(sigs.len(), 2);
      
    }
}
