// Copyright 2018-2019 Chainpool.

use super::*;
use xstaking::{VoteWeightBase, VoteWeightBaseV1, WeightFactors, WeightType};
use xsupport::{error, trace};

impl<T: Trait> ComputeWeight<T::AccountId> for Module<T> {
    type Claimee = Token;
    fn prepare_claimer_weight_factors(
        who: &T::AccountId,
        target: &Self::Claimee,
        current_block: u64,
    ) -> WeightFactors {
        let dr_key = (who.clone(), target.clone());

        match Self::try_get_deposit_record(&dr_key) {
            Ok(mut d) => {
                let record = DepositRecord::<T>::new(who, target, &mut d);
                (
                    WeightType::U64(record.last_acum_weight()),
                    record.amount(),
                    current_block - record.last_acum_weight_update(),
                )
            }
            Err(mut d1) => {
                let record_v1 = DepositRecordV1::<T>::new(who, target, &mut d1);
                (
                    WeightType::U128(record_v1.last_acum_weight()),
                    record_v1.amount(),
                    current_block - record_v1.last_acum_weight_update(),
                )
            }
        }
    }

    fn prepare_claimee_weight_factors(target: &Self::Claimee, current_block: u64) -> WeightFactors {
        match Self::try_get_psedu_intention_profs(target) {
            Ok(mut p) => {
                let prof = PseduIntentionProfs::<T>::new(target, &mut p);

                (
                    WeightType::U64(prof.last_acum_weight()),
                    prof.amount(),
                    current_block - prof.last_acum_weight_update(),
                )
            }
            Err(mut p1) => {
                let prof_v1 = PseduIntentionProfsV1::<T>::new(target, &mut p1);
                (
                    WeightType::U128(prof_v1.last_acum_weight()),
                    prof_v1.amount(),
                    current_block - prof_v1.last_acum_weight_update(),
                )
            }
        }
    }
}

impl<T: Trait> Claim<T::AccountId, T::Balance> for Module<T> {
    type Claimee = Token;

    fn allocate_dividend(
        claimer: &T::AccountId,
        claimee: &Token,
        claimee_jackpot: &T::AccountId,
        dividend: T::Balance,
    ) -> Result {
        let referral_or_council = xtokens::Module::<T>::referral_or_council_of(claimer, claimee);
        // 10% claim distributes to the depositor's referral.
        let to_referral_or_council = T::Balance::sa(dividend.as_() / 10);

        trace!(
            target: "claim",
            "[before moving to referral_or_council] should move {:?} from the jackpot to referral_or_council, current jackpot_balance: {:?}",
            to_referral_or_council,
            xassets::Module::<T>::pcx_free_balance(claimee_jackpot)
        );

        xassets::Module::<T>::pcx_move_free_balance(
                    claimee_jackpot,
                    &referral_or_council,
                    to_referral_or_council,
                )
                    .map_err(|e| {
                        error!(
                            "[allocate cross miner dividend] fail to move {:?} from jackpot_addr to referral_or_council, current jackpot_balance: {:?}",
                            to_referral_or_council,
                            xassets::Module::<T>::pcx_free_balance(claimee_jackpot)
                        );
                        e.info()
                    })?;

        trace!(target: "claim", "[after moving to referral_or_council] jackpot_balance: {:?}", xassets::Module::<T>::pcx_free_balance(claimee_jackpot));

        trace!(
            target: "claim",
            "[before moving to cross miner] should move {:?} from jackpot to depositor, current jackpot_balance: {:?}",
            dividend - to_referral_or_council,
            xassets::Module::<T>::pcx_free_balance(claimee_jackpot)
        );

        xassets::Module::<T>::pcx_move_free_balance(
                    claimee_jackpot,
                    claimer,
                    dividend - to_referral_or_council,
                )
                    .map_err(|e| {
                        error!(
                            "[allocate cross miner dividend] fail to move {:?} from jackpot_addr to some depositor, current jackpot_balance: {:?}",
                            dividend - to_referral_or_council,
                            xassets::Module::<T>::pcx_free_balance(claimee_jackpot),
                        );
                        e.info()
                    })?;

        trace!(target: "claim", "[after moving to cross miner] jackpot_balance: {:?}", xassets::Module::<T>::pcx_free_balance(claimee_jackpot));

        Ok(())
    }

    fn claim(claimer: &T::AccountId, claimee: &Self::Claimee) -> Result {
        let current_block = <system::Module<T>>::block_number();

        let ((source_vote_weight, source_overflow), (target_vote_weight, target_overflow)) =
            <Self as ComputeWeight<T::AccountId>>::settle_weight_on_claim(
                claimer,
                claimee,
                current_block.as_(),
            )?;

        let claimee_jackpot = T::DetermineTokenJackpotAccountId::accountid_for_unsafe(claimee);

        let dividend = xstaking::compute_dividend::<T>(
            source_vote_weight,
            target_vote_weight,
            &claimee_jackpot,
        );

        xtokens::Module::<T>::can_claim(claimer, claimee, dividend, current_block)?;

        Self::allocate_dividend(claimer, claimee, &claimee_jackpot, dividend)?;

        xtokens::Module::<T>::apply_state_change_on_claim(
            claimer,
            claimee,
            target_vote_weight - source_vote_weight,
            current_block,
        );

        let key = (claimer.clone(), claimee.clone());
        <LastClaimOf<T>>::insert(&key, current_block);

        xtokens::Module::<T>::deposit_claim_event(
            (source_vote_weight, source_overflow),
            (target_vote_weight, target_overflow),
            claimer,
            claimee,
            dividend,
        );

        Ok(())
    }
}

impl<T: Trait> OnAssetChanged<T::AccountId, T::Balance> for Module<T> {
    fn on_move_before(
        token: &Token,
        from: &T::AccountId,
        _: AssetType,
        to: &T::AccountId,
        _: AssetType,
        _value: T::Balance,
    ) {
        // Exclude PCX and asset type changes on same account.
        if <xassets::Module<T> as ChainT>::TOKEN == token.as_slice() || from.clone() == to.clone() {
            return;
        }

        let current_block = <system::Module<T>>::block_number();
        Self::try_init_receiver_vote_weight(to, token, current_block);

        Self::update_depositor_vote_weight(from, token, current_block);
        Self::update_depositor_vote_weight(to, token, current_block);
    }

    fn on_move(
        _token: &Token,
        _from: &T::AccountId,
        _: AssetType,
        _to: &T::AccountId,
        _: AssetType,
        _value: T::Balance,
    ) -> result::Result<(), AssetErr> {
        Ok(())
    }

    fn on_issue_before(target: &Token, source: &T::AccountId) {
        // Exclude PCX
        if <xassets::Module<T> as ChainT>::TOKEN == target.as_slice() {
            return;
        }

        let current_block = <system::Module<T>>::block_number();
        Self::try_init_receiver_vote_weight(source, target, current_block);

        debug!(
            "[on_issue_before] deposit_records: ({:?}, {:?}) = {:?}",
            token!(target),
            source,
            Self::deposit_records((source.clone(), target.clone()))
        );

        Self::update_bare_vote_weight(source, target, current_block);
    }

    fn on_issue(target: &Token, source: &T::AccountId, value: T::Balance) -> Result {
        // Exclude PCX
        if <xassets::Module<T> as ChainT>::TOKEN == target.as_slice() {
            return Ok(());
        }

        debug!(
            "[on_issue] token: {:?}, who: {:?}, vlaue: {:?}",
            token!(target),
            source,
            value
        );

        Self::issue_reward(source, target, value)
    }

    fn on_destroy_before(target: &Token, source: &T::AccountId) {
        let current_block = <system::Module<T>>::block_number();
        Self::update_bare_vote_weight(source, target, current_block);
    }

    fn on_destroy(_target: &Token, _source: &T::AccountId, _value: T::Balance) -> Result {
        Ok(())
    }
}
