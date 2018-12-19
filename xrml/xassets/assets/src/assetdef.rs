// Copyright 2018 Chainpool.

use rstd::prelude::*;
use rstd::result::Result as StdResult;

use runtime_support::dispatch::Result;

pub type TokenString = &'static [u8];
pub type DescString = TokenString;
pub type Token = Vec<u8>;
pub type Desc = Vec<u8>;
pub type Precision = u16;

pub trait ChainT {
    const TOKEN: &'static [u8];
    fn chain() -> Chain;
    fn check_addr(_addr: &[u8], _ext: &[u8]) -> Result {
        Ok(())
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum Chain {
    PCX,
    BTC,
    ETH,
}

impl Default for Chain {
    fn default() -> Self {
        Chain::PCX
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub struct Asset {
    token: Token,
    chain: Chain,
    precision: Precision,
    desc: Desc,
}

impl Asset {
    pub fn new(
        token: Token,
        chain: Chain,
        precision: Precision,
        desc: Desc,
    ) -> StdResult<Self, &'static str> {
        let a = Asset {
            token,
            chain,
            precision,
            desc,
        };
        a.is_valid()?;
        Ok(a)
    }
    pub fn is_valid(&self) -> Result {
        is_valid_token(&self.token)?;
        is_valid_desc(&self.desc)
    }

    pub fn token(&self) -> Token {
        self.token.clone()
    }
    pub fn chain(&self) -> Chain {
        self.chain
    }
    pub fn desc(&self) -> Desc {
        self.desc.clone()
    }
    pub fn set_desc(&mut self, desc: Desc) {
        self.desc = desc
    }
    pub fn precision(&self) -> Precision {
        self.precision
    }
}

const MAX_TOKEN_LEN: usize = 32;
const MAX_DESC_LEN: usize = 128;

pub fn is_valid_token(v: &[u8]) -> Result {
    if v.len() > MAX_TOKEN_LEN || v.len() == 0 {
        Err("symbol length too long or zero")
    } else {
        for c in v.iter() {
            // allow number (0x30~0x39), capital letter (0x41~0x5A), small letter (0x61~0x7A), - 0x2D, . 0x2E, | 0x7C,  ~ 0x7E
            if (*c >= 0x30 && *c <= 0x39) // number
                || (*c >= 0x41 && *c <= 0x5A) // capital
                || (*c >= 0x61 && *c <= 0x7A) // small
                || (*c == 0x2D) // -
                || (*c == 0x2E) // .
                || (*c == 0x7C) // |
                || (*c == 0x7E)
            // ~
            {
                continue;
            } else {
                return Err("not a valid symbol char for number, capital/small letter or '-', '.', '|', '~'");
            }
        }
        Ok(())
    }
}

pub fn is_valid_desc(v: &[u8]) -> Result {
    if v.len() > MAX_DESC_LEN {
        Err("token desc length too long")
    } else {
        for c in v.iter() {
            // ascii visible char
            if *c >= 20 && *c <= 0x7E {
                continue;
            } else {
                return Err("not a valid ascii visible char");
            }
        }
        Ok(())
    }
}
