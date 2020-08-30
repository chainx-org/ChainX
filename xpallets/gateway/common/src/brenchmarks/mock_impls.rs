use super::*;

// hack for compiling error
pub struct MockBitcoin<T: xpallet_gateway_bitcoin::Trait>(sp_std::marker::PhantomData<T>);
mod bitcoin {
    use super::*;
    use crate::trustees::bitcoin::{BtcTrusteeAddrInfo, BtcTrusteeType};
    use crate::types::TrusteeSessionInfo;

    impl<T: xpallet_gateway_bitcoin::Trait> ChainT<BalanceOf<T>> for MockBitcoin<T> {
        const ASSET_ID: u32 = X_BTC;

        fn chain() -> Chain {
            Chain::Bitcoin
        }

        fn check_addr(_: &[u8], _: &[u8]) -> DispatchResult {
            Ok(())
        }

        fn withdrawal_limit(
            asset_id: &u32,
        ) -> Result<WithdrawalLimit<BalanceOf<T>>, DispatchError> {
            xpallet_gateway_bitcoin::Module::<T>::withdrawal_limit(asset_id)
        }
    }

    impl<T: xpallet_gateway_bitcoin::Trait>
        TrusteeForChain<T::AccountId, BtcTrusteeType, BtcTrusteeAddrInfo> for MockBitcoin<T>
    {
        fn check_trustee_entity(raw_addr: &[u8]) -> Result<BtcTrusteeType, DispatchError> {
            let trustee_type =
                BtcTrusteeType::try_from(raw_addr.to_vec()).map_err(|_| "InvalidPublicKey")?;
            Ok(trustee_type)
        }

        fn generate_trustee_session_info(
            props: Vec<(T::AccountId, TrusteeIntentionProps<BtcTrusteeType>)>,
            _: TrusteeInfoConfig,
        ) -> Result<TrusteeSessionInfo<T::AccountId, BtcTrusteeAddrInfo>, DispatchError> {
            let len = props.len();
            Ok(TrusteeSessionInfo {
                trustee_list: props.into_iter().map(|(a, _)| a).collect::<_>(),
                threshold: len as u16,
                hot_address: BtcTrusteeAddrInfo {
                    addr: vec![],
                    redeem_script: vec![],
                },
                cold_address: BtcTrusteeAddrInfo {
                    addr: vec![],
                    redeem_script: vec![],
                },
            })
        }
    }
}
