use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeBalanceInfo<AccountId, Balance> {
    pub free: Balance,
    pub who: AccountId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nomination<AccountId, Balance> {
    pub nominee: AccountId,
    pub nomination: Balance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NominatorInfo<AccountId, Balance> {
    pub nominator: AccountId,
    pub nominations: Vec<Nomination<AccountId, Balance>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo<AccountId, Balance> {
    pub who: AccountId,
    #[serde(with = "xp_rpc::serde_text")]
    pub referral_id: Vec<u8>,
    pub total_nomination: Balance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XStakingParams<AccountId, Balance> {
    pub validators: Vec<ValidatorInfo<AccountId, Balance>>,
    pub nominators: Vec<NominatorInfo<AccountId, Balance>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllParams<AccountId, Balance, AssetBalanceOf, StakingBalanceOf> {
    pub balances: Vec<FreeBalanceInfo<AccountId, Balance>>,
    pub xassets: Vec<FreeBalanceInfo<AccountId, AssetBalanceOf>>,
    pub xstaking: XStakingParams<AccountId, StakingBalanceOf>,
}

impl<AccountId, Balance, AssetBalanceOf, StakingBalanceOf> Default
    for AllParams<AccountId, Balance, AssetBalanceOf, StakingBalanceOf> {
    fn default() -> Self {
        AllParams {
            balances: vec![],
            xassets: vec![],
            xstaking: XStakingParams {
                validators: vec![],
                nominators: vec![]
            }
        }
    }
}
