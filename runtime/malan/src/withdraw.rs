#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use core::marker::PhantomData;
use fp_evm::{
    Context, ExitRevert, ExitSucceed, PrecompileFailure, PrecompileOutput, PrecompileResult,
};
use frame_support::log;
use pallet_evm::{AddressMapping, Precompile};
use sp_core::{hexdisplay::HexDisplay, H160, U256};
use sp_runtime::{traits::UniqueSaturatedInto, AccountId32};
use sp_std::vec;

const MIN_BTC_TRANSFER_VALUE: u128 = 10_000_000_000;
const BASE_GAS_COST: u64 = 100_000;

pub struct Withdraw<
    T: xpallet_assets_bridge::Config
        + xpallet_gateway_common::Config
        + xpallet_gateway_records::Config,
> {
    _marker: PhantomData<T>,
}

impl<
        T: xpallet_assets_bridge::Config
            + xpallet_gateway_common::Config
            + xpallet_gateway_records::Config,
    > Withdraw<T>
{
    fn process(caller: &H160, input: &[u8]) -> Result<(), PrecompileFailure> {
        match input.first() {
            // Withdraw BTC
            Some(&0) if input.len() >= 67 && input.len() <= 95 => {
                // input = (flag, 1 byte) + value(32 bytes) + to(btc address, 34-62 bytes)
                // https://www.doubloin.com/learn/how-long-are-bitcoin-addresses
                log::debug!(target: "evm-withdraw", "btc: call");

                Self::process_withdraw_btc(caller, &input[1..]).map_err(|err| {
                    log::warn!(target: "evm-withdraw", "btc: err = {:?}", err);
                    err
                })?;

                log::debug!(target: "evm-withdraw", "btc: success");

                Ok(())
            }
            // Withdraw PCX
            Some(&1) if input.len() == 65 => {
                // input = (flag, 1 byte) + value(32 bytes) + to(substrate pubkey, 32 bytes)

                log::debug!(target: "evm-withdraw", "pcx: call");

                Self::process_withdraw_pcx(caller, &input[1..]).map_err(|err| {
                    log::warn!(target: "evm-withdraw", "pcx: err = {:?}", err);
                    err
                })?;

                log::debug!(target: "evm-withdraw", "pcx: success");

                Ok(())
            }
            _ => {
                log::warn!(target: "evm-withdraw", "invalid input: {:?}", input);

                Err(PrecompileFailure::Revert {
                    exit_status: ExitRevert::Reverted,
                    output: "invalid withdraw(0x403) input".into(),
                    cost: BASE_GAS_COST,
                })
            }
        }
    }

    fn account_from_pubkey(pubkey: &[u8]) -> Result<T::AccountId, PrecompileFailure> {
        frame_support::ensure!(
            pubkey.len() == 32,
            PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted,
                output: "invalid chainx pubkey".into(),
                cost: BASE_GAS_COST
            }
        );

        let mut target = [0u8; 32];
        target[0..32].copy_from_slice(&pubkey[0..32]);

        T::AccountId::decode(&mut &AccountId32::new(target).encode()[..]).map_err(|_| {
            PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted,
                output: "decode AccountId32 failed".into(),
                cost: BASE_GAS_COST,
            }
        })
    }

    fn balance(value: &[u8], is_btc: bool) -> Result<u128, PrecompileFailure> {
        frame_support::ensure!(
            value.len() == 32,
            PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted,
                output: "invalid balance".into(),
                cost: BASE_GAS_COST
            }
        );

        let mut balance = U256::from_big_endian(&value[0..32]).low_u128();

        if balance == 0 {
            return Err(PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted,
                output: "zero balance".into(),
                cost: BASE_GAS_COST,
            });
        }

        if is_btc {
            // evm balance decimals=18, wasm balance decimals=8
            if balance < MIN_BTC_TRANSFER_VALUE {
                return Err(PrecompileFailure::Revert {
                    exit_status: ExitRevert::Reverted,
                    output: "balance < 10 Gwei".into(),
                    cost: BASE_GAS_COST,
                });
            }

            balance = balance
                .checked_div(MIN_BTC_TRANSFER_VALUE)
                .unwrap_or(u128::MAX);
        }

        Ok(balance)
    }

    fn process_withdraw_pcx(caller: &H160, input: &[u8]) -> Result<(), PrecompileFailure> {
        let balance = Self::balance(&input[0..32], false)?;
        let to = Self::account_from_pubkey(&input[32..64])?;

        log::debug!(target: "evm-withdraw", "from(evm): {:?}", caller);
        log::debug!(target: "evm-withdraw", "to(pcx): {:?}", HexDisplay::from(&to.encode()));
        log::debug!(target: "evm-withdraw", "value(sub): {:?}", balance);

        xpallet_assets_bridge::Pallet::<T>::withdraw_pcx_from_evm(*caller, to, balance).map_err(
            |err| {
                log::debug!(target: "evm-withdraw", "withdraw_pcx: {:?}", err);

                PrecompileFailure::Revert {
                    exit_status: ExitRevert::Reverted,
                    output: "withdraw pcx failed".into(),
                    cost: BASE_GAS_COST,
                }
            },
        )
    }

    fn process_withdraw_btc(caller: &H160, input: &[u8]) -> Result<(), PrecompileFailure> {
        let from = T::AddressMapping::into_account_id(*caller);
        let balance = Self::balance(&input[0..32], true)?;
        let btc_addr = &input[32..];

        log::debug!(target: "evm-withdraw", "from(evm): {:?}", caller);
        log::debug!(target: "evm-withdraw", "to(btc): {:?}", btc_addr);
        log::debug!(target: "evm-withdraw", "value(sub): {:?}", balance);

        xpallet_assets_bridge::Pallet::<T>::swap_btc_to_xbtc(*caller, balance).map_err(|err| {
            log::debug!(target: "evm-withdraw", "btc_to_xbtc: {:?}", err);

            PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted,
                output: "swap btc failed".into(),
                cost: BASE_GAS_COST,
            }
        })?;

        xpallet_gateway_common::Pallet::<T>::verify_withdrawal(
            1,
            balance.unique_saturated_into(),
            btc_addr,
            &Default::default(),
        )
        .map_err(|err| {
            log::debug!(target: "evm-withdraw", "verify_withdrawal: {:?}", err);

            PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted,
                output: "verify withdrawal failed".into(),
                cost: BASE_GAS_COST,
            }
        })?;

        xpallet_gateway_records::Pallet::<T>::withdraw(
            &from,
            1,
            balance.unique_saturated_into(),
            btc_addr.to_vec(),
            Default::default(),
        )
        .map_err(|err| {
            log::debug!(target: "evm-withdraw", "xbtc withdraw: {:?}", err);

            PrecompileFailure::Revert {
                exit_status: ExitRevert::Reverted,
                output: "xbtc withdraw failed".into(),
                cost: BASE_GAS_COST,
            }
        })?;

        Ok(())
    }
}

impl<T> Precompile for Withdraw<T>
where
    T: xpallet_assets_bridge::Config
        + xpallet_gateway_common::Config
        + xpallet_gateway_records::Config,
    T::AccountId: Decode,
{
    fn execute(
        input: &[u8],
        _target_gas: Option<u64>,
        context: &Context,
        _: bool,
    ) -> PrecompileResult {
        log::debug!(target: "evm-withdraw", "caller: {:?}", context.caller);

        Self::process(&context.caller, input).map(|_| {
            // Refer: https://github.com/rust-ethereum/ethabi/blob/master/ethabi/src/encoder.rs#L144
            let mut out = vec![0u8; 32];
            out[31] = 1u8;

            Ok(PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                cost: BASE_GAS_COST,
                output: out.to_vec(),
                logs: Default::default(),
            })
        })?
    }
}
