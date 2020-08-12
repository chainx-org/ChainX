// Copyright 2017-2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Substrate RPC interfaces.

#![allow(clippy::needless_return)]
#![allow(clippy::type_complexity)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::single_match)]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::identity_conversion)]
#![allow(clippy::redundant_closure)]
//#![warn(missing_docs)]

mod errors;
mod helpers;
mod subscriptions;

pub use subscriptions::Subscriptions;

pub mod author;
pub mod chain;
pub mod chainx;
pub mod metadata;
pub mod state;
pub mod system;

use jsonrpc_core as rpc;

pub use chainx::set_cache_flag;
