use rstd::result;

use xassets::Chain;
use xbridge_common::{
    traits::{TrusteeMultiSig, TrusteeSession},
    types::{TrusteeIntentionProps, TrusteeSessionInfo},
};
use xsupport::{error, warn};

// for bitcoin trustee
pub use btc_keys::Public as BitcoinPublic;
pub use xbitcoin::TrusteeAddrInfo as BitcoinTrusteeAddrInfo;

use super::{Module, Trait};

// bitcoin trustee type
pub type BitcoinTrusteeType = BitcoinPublic;

pub type BitcoinTrusteeIntentionProps = TrusteeIntentionProps<BitcoinTrusteeType>;

pub type BitcoinTrusteeSessionInfo<AccountId> =
    TrusteeSessionInfo<AccountId, BitcoinTrusteeAddrInfo>;

/// for bitcoin
impl<T: Trait> TrusteeSession<T::AccountId, BitcoinTrusteeAddrInfo> for Module<T> {
    fn trustee_session(
        number: u32,
    ) -> result::Result<TrusteeSessionInfo<T::AccountId, BitcoinTrusteeAddrInfo>, &'static str>
    {
        Self::bitcoin_trustee_session_info_of(number).ok_or_else(|| {
            error!(
                "[trustee_session]|not found info for this session|chain:{:?}|number:{:}",
                Chain::Bitcoin,
                number
            );
            "not found info for this session"
        })
    }

    fn current_trustee_session(
    ) -> result::Result<TrusteeSessionInfo<T::AccountId, BitcoinTrusteeAddrInfo>, &'static str>
    {
        let number = Self::current_session_number(Chain::Bitcoin);
        Self::trustee_session(number)
    }

    fn last_trustee_session(
    ) -> result::Result<TrusteeSessionInfo<T::AccountId, BitcoinTrusteeAddrInfo>, &'static str>
    {
        let number = Self::last_session_number(Chain::Bitcoin);
        Self::trustee_session(number).map_err(|e| {
            warn!(
                "[last_trustee_session]|last trustee session not exist yet for this chain|Chain:{:?}",
                Chain::Bitcoin
            );
            e
        })
    }
}

pub struct BitcoinTrusteeMultiSig<T: Trait>(::rstd::marker::PhantomData<T>);

impl<T: Trait> TrusteeMultiSig<T::AccountId> for BitcoinTrusteeMultiSig<T> {
    fn multisig_for_trustees() -> T::AccountId {
        Module::<T>::trustee_multisig_addr(Chain::Bitcoin)
    }
}
