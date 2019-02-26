// Copyright 2018 Chainpool.

use rstd::prelude::*;
use super::{Codec, system, balances};


ype TransferT<T> = Transfer<<T as system::Trait>::AccountId, <T as balances::Trait>::Balance>;