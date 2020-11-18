// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use super::*;

pub use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;

const SEED: u32 = 0;

/// Grab a funded user.
pub fn create_funded_user<T: Trait>(string: &'static str, n: u32, value: u32) -> T::AccountId {
    let user = account(string, n, SEED);
    let balance = value.into();
    T::Currency::make_free_balance_be(&user, balance);
    // ensure T::CurrencyToVote will work correctly.
    T::Currency::issue(balance);
    user
}

fn b_bond<T: Trait>(nominator: T::AccountId, validator: T::AccountId, value: u32) {
    let validator_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(validator);
    assert!(crate::Module::<T>::bond(
        RawOrigin::Signed(nominator).into(),
        validator_lookup,
        value.into(),
    )
    .is_ok());
}

pub fn create_validator<T: Trait>(string: &'static str, n: u32, value: u32) -> T::AccountId {
    let validator = create_funded_user::<T>(string, n, value);
    assert!(crate::Module::<T>::register(
        RawOrigin::Signed(validator.clone()).into(),
        n.to_be_bytes().to_vec(),
        value.into()
    )
    .is_ok());
    validator
}

benchmarks! {
    _{
        // User account seed
        let u in 0 .. 1000 => ();
    }

    register {
        let validator = create_funded_user::<T>("validator", u, 100);
        let referral_id = (u as u32).to_be_bytes();
    }: _(RawOrigin::Signed(validator.clone()), referral_id.to_vec(), 10.into())
    verify {
        assert!(Validators::<T>::contains_key(validator));
    }

    bond {
        let nominator = create_funded_user::<T>("nominator", u, 100);
        let validator: T::AccountId = create_validator::<T>("validator", 2, 1000);
        let validator_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(validator.clone());
    }: _(RawOrigin::Signed(nominator.clone()), validator_lookup, 10.into())
    verify {
        assert!(Nominations::<T>::contains_key(nominator, validator));
    }

    unbond {
        let validator: T::AccountId = create_validator::<T>("validator", 2, 100);
        let validator_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(validator.clone());
    }: _(RawOrigin::Signed(validator.clone()), validator_lookup, 10.into())
    verify {
        assert!(Module::<T>::bonded_to(&validator, &validator) == 90.into());
    }

    unlock_unbonded_withdrawal {
        let validator: T::AccountId = create_validator::<T>("validator", 2, 100);
        let validator_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(validator.clone());

        Module::<T>::set_validator_bonding_duration(RawOrigin::Root.into(), 0.into())?;

        Module::<T>::unbond(
            RawOrigin::Signed(validator.clone()).into(),
            validator_lookup.clone(),
            20.into(),
        )?;

        let block_number: T::BlockNumber = frame_system::Module::<T>::block_number();
        frame_system::Module::<T>::set_block_number(block_number + 1.into());

    }: _(RawOrigin::Signed(validator.clone()), validator_lookup, 0)
    verify {
        assert!(Module::<T>::bonded_to(&validator, &validator) == 80.into());
        assert!(Module::<T>::staked_of(&validator)  == 80.into());
    }

    rebond {
        let nominator = create_funded_user::<T>("nominator", u, 100);
        let validator1: T::AccountId = create_validator::<T>("validator1", 2, 100);
        let validator2: T::AccountId = create_validator::<T>("validator2", 3, 100);
        b_bond::<T>(nominator.clone(), validator1.clone(), 30);
        let validator1_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(validator1.clone());
        let validator2_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(validator2.clone());
    }: _(RawOrigin::Signed(nominator.clone()), validator1_lookup, validator2_lookup, 10.into())
    verify {
        assert!(Module::<T>::bonded_to(&nominator, &validator1) == 20.into());
        assert!(Module::<T>::bonded_to(&nominator, &validator2) == 10.into());
    }

    claim {
        let validator: T::AccountId = create_validator::<T>("validator", 2, 1000);
        let validator_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(validator.clone());

        let validator_pot = T::DetermineRewardPotAccount::reward_pot_account_for(&validator);

        let pot_balance = 50;
        T::Currency::make_free_balance_be(&validator_pot, pot_balance.into());
        T::Currency::issue(pot_balance.into());

        let balance_before = T::Currency::free_balance(&validator);

        let block_number: T::BlockNumber = frame_system::Module::<T>::block_number();
        frame_system::Module::<T>::set_block_number(block_number + 1.into());
    }: _(RawOrigin::Signed(validator.clone()), validator_lookup)
    verify {
        assert!(T::Currency::total_balance(&validator) == balance_before + pot_balance.into());
    }

    chill {
        let validator: T::AccountId = create_validator::<T>("validator", 2, 1000);
        if !Module::<T>::is_validator(&validator) {
            Module::<T>::register(RawOrigin::Signed(validator.clone()).into(), (u as u32).to_be_bytes().to_vec(), 100.into())?;
        }
    }: _(RawOrigin::Signed(validator.clone()))
    verify {
        assert!(Module::<T>::is_chilled(&validator));
    }

    validate {
        let validator: T::AccountId = create_validator::<T>("validator", 2, 1000);
        if !Module::<T>::is_validator(&validator) {
            Module::<T>::register(RawOrigin::Signed(validator.clone()).into(), (u as u32).to_be_bytes().to_vec(), 100.into())?;
        }
        Module::<T>::chill(RawOrigin::Signed(validator.clone()).into())?;
    }: _(RawOrigin::Signed(validator.clone()))
    verify {
        assert!(Module::<T>::is_active(&validator));
    }

    set_validator_count {
        let c = 1000;
    }: _(RawOrigin::Root, c)
    verify {
        assert_eq!(ValidatorCount::get(), c);
    }

    set_minimum_validator_count {
        let c = 1000;
    }: _(RawOrigin::Root, c)
    verify {
        assert_eq!(MinimumValidatorCount::get(), c);
    }

    set_bonding_duration {
        let c = 100;
    }: _(RawOrigin::Root, c.into())
    verify {
        assert_eq!(BondingDuration::<T>::get(), c.into());
    }

    set_validator_bonding_duration {
        let c = 1000;
    }: _(RawOrigin::Root, c.into())
    verify {
        assert_eq!(ValidatorBondingDuration::<T>::get(), c.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{ExtBuilder, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(test_benchmark_register::<Test>());
            assert_ok!(test_benchmark_bond::<Test>());
            assert_ok!(test_benchmark_unbond::<Test>());
            assert_ok!(test_benchmark_unlock_unbonded_withdrawal::<Test>());
            assert_ok!(test_benchmark_rebond::<Test>());
            assert_ok!(test_benchmark_claim::<Test>());
            assert_ok!(test_benchmark_chill::<Test>());
            assert_ok!(test_benchmark_validate::<Test>());
            assert_ok!(test_benchmark_set_validator_count::<Test>());
            assert_ok!(test_benchmark_set_minimum_validator_count::<Test>());
            assert_ok!(test_benchmark_set_bonding_duration::<Test>());
            assert_ok!(test_benchmark_set_validator_bonding_duration::<Test>());
        });
    }
}
