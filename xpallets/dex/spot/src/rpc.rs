use super::*;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Saturating, RuntimeDebug};
use xpallet_support::{RpcBalance, RpcPrice};

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct PairInfo<RpcPrice> {
    #[cfg_attr(feature = "std", serde(flatten))]
    pub profile: TradingPairProfile,
    #[cfg_attr(feature = "std", serde(flatten))]
    pub handicap: RpcHandicap<RpcPrice>,
    /// The maximum valid bid price.
    pub max_valid_bid: RpcPrice,
    /// The minimum valid ask price.
    pub min_valid_ask: RpcPrice,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, Default, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RpcHandicap<RpcPrice> {
    pub highest_bid: RpcPrice,
    pub lowest_ask: RpcPrice,
}

impl<Price> From<Handicap<Price>> for RpcHandicap<RpcPrice<Price>> {
    fn from(handicap: Handicap<Price>) -> Self {
        Self {
            highest_bid: handicap.highest_bid.into(),
            lowest_ask: handicap.lowest_offer.into(),
        }
    }
}

/// Immutable information of an order.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RpcOrderProperty<PairId, AccountId, RpcBalance, RpcPrice, BlockNumber> {
    /// The order identifier.
    pub id: OrderId,
    /// The direction of order.
    pub side: Side,
    /// The price of order.
    pub price: RpcPrice,
    /// The amount of order, measured in the base currency.
    pub amount: RpcBalance,
    /// The trading pair identifier.
    pub pair_id: PairId,
    /// The account that submitted the order.
    pub submitter: AccountId,
    /// The type of order.
    pub order_type: OrderType,
    /// Block number at which the order is created.
    pub created_at: BlockNumber,
}

impl<PairId, AccountId, Amount, Price, BlockNumber>
    From<OrderProperty<PairId, AccountId, Amount, Price, BlockNumber>>
    for RpcOrderProperty<PairId, AccountId, RpcBalance<Amount>, RpcPrice<Price>, BlockNumber>
{
    fn from(props: OrderProperty<PairId, AccountId, Amount, Price, BlockNumber>) -> Self {
        let price: RpcPrice<Price> = props.price.into();
        let amount: RpcBalance<Amount> = props.amount.into();
        Self {
            id: props.id,
            side: props.side,
            price,
            amount,
            pair_id: props.pair_id,
            submitter: props.submitter,
            order_type: props.order_type,
            created_at: props.created_at,
        }
    }
}

/// Details of an order.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct RpcOrder<PairId, AccountId, RpcBalance, RpcPrice, BlockNumber> {
    /// Immutable details of the order.
    pub props: RpcOrderProperty<PairId, AccountId, RpcBalance, RpcPrice, BlockNumber>,
    /// Status of the order.
    pub status: OrderStatus,
    /// The amount of unexecuted, measured by the **quote** currency.
    ///
    /// While (props.amount() - already_filled) can be expressed as
    /// the remaining amount as well, it's measured by the base currency.
    pub remaining: RpcBalance,
    /// Indices of all executed transaction records.
    pub executed_indices: Vec<TradingHistoryIndex>,
    /// The amount of executed, measured by the **base** currency.
    pub already_filled: RpcBalance,
    /// Block number at which the order details updated.
    pub last_update_at: BlockNumber,
}

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct Depth<RpcPrice, RpcBalance> {
    /// List of bids in pair of (price, quantity).
    pub bids: Vec<(RpcPrice, RpcBalance)>,
    /// List of asks in pair of (price, quantity).
    pub asks: Vec<(RpcPrice, RpcBalance)>,
}

impl<T: Trait> Module<T> {
    /// Returns the range of a valid quotation for a trading pair.
    fn get_quotation_range(profile: &TradingPairProfile) -> (T::Price, T::Price) {
        let handicap = Self::handicap_of(profile.id);
        let pair_fluctuation: T::Price = profile.calc_fluctuation::<T>().saturated_into();
        let max_valid_bid = if !handicap.lowest_offer.is_zero() {
            handicap.lowest_offer + pair_fluctuation
        } else {
            Zero::zero()
        };
        let min_valid_ask = if handicap.highest_bid > pair_fluctuation {
            handicap.highest_bid - pair_fluctuation
        } else {
            profile.tick().saturated_into()
        };
        (min_valid_ask, max_valid_bid)
    }

    /// Get the overall info of all trading pairs.
    pub fn trading_pairs() -> Vec<PairInfo<RpcPrice<T::Price>>> {
        let pair_count = Self::trading_pair_count();
        let mut pairs = Vec::with_capacity(pair_count as usize);
        for pair_id in 0..pair_count {
            if let Some(profile) = Self::trading_pair_of(pair_id) {
                let (min_valid_ask, max_valid_bid) = Self::get_quotation_range(&profile);
                let handicap = Self::handicap_of(pair_id);
                let handicap: RpcHandicap<RpcPrice<T::Price>> = handicap.into();
                pairs.push(PairInfo {
                    profile,
                    handicap,
                    max_valid_bid: max_valid_bid.into(),
                    min_valid_ask: min_valid_ask.into(),
                });
            }
        }
        pairs
    }

    /// Get the orders of an account.
    pub fn orders(
        who: T::AccountId,
    ) -> Vec<
        RpcOrder<
            TradingPairId,
            T::AccountId,
            RpcBalance<BalanceOf<T>>,
            RpcPrice<T::Price>,
            T::BlockNumber,
        >,
    > {
        // TODO: into one page
        OrderInfoOf::<T>::iter_prefix_values(who)
            .filter_map(|order| {
                let props: RpcOrderProperty<
                    TradingPairId,
                    T::AccountId,
                    RpcBalance<BalanceOf<T>>,
                    RpcPrice<T::Price>,
                    T::BlockNumber,
                > = order.props.into();
                Some(RpcOrder {
                    props,
                    status: order.status,
                    remaining: order.remaining.into(),
                    executed_indices: order.executed_indices,
                    already_filled: order.already_filled.into(),
                    last_update_at: order.last_update_at,
                })
            })
            .collect()
    }

    /// Returns the sum of unfilled quantities at `price` of a trading pair `pair_id`.
    fn get_commulative_qty(pair_id: TradingPairId, price: T::Price) -> u128 {
        QuotationsOf::<T>::get(pair_id, price.saturated_into::<T::Price>())
            .iter()
            .filter_map(|(trader, order_id)| OrderInfoOf::<T>::get(trader, order_id))
            .map(|order| {
                order
                    .amount()
                    .saturating_sub(order.already_filled)
                    .saturated_into::<u128>()
            })
            .sum()
    }

    /// Get the depth of a trading pair given the depth size.
    pub fn depth(
        pair_id: TradingPairId,
        depth_size: u32,
    ) -> Option<Depth<RpcPrice<T::Price>, RpcBalance<BalanceOf<T>>>> {
        Self::trading_pair_of(pair_id).map(|pair| {
            let handicap = Self::handicap_of(pair_id);

            let (lowest_ask, highest_bid) = (handicap.lowest_offer, handicap.highest_bid);
            let (min_valid_ask, max_valid_bid) = Self::get_quotation_range(&pair);

            let step = pair.tick().saturated_into::<u128>();

            let generic_depth = |start: T::Price, end: T::Price| {
                let start = start.saturated_into::<u128>();
                let end = end.saturated_into::<u128>();
                (0..)
                    .map(|x| start + step * x)
                    .take_while(|&x| x <= end)
                    .filter_map(|price| {
                        let cummulative_qty =
                            Self::get_commulative_qty(pair_id, price.saturated_into());
                        if cummulative_qty.is_zero() {
                            None
                        } else {
                            let price: T::Price = price.saturated_into();
                            let cummulative_qty: BalanceOf<T> = cummulative_qty.saturated_into();
                            Some((price.into(), cummulative_qty.into()))
                        }
                    })
                    .take(depth_size as usize)
                    .collect::<Vec<_>>()
            };

            let asks = generic_depth(lowest_ask, max_valid_bid);
            let bids = generic_depth(min_valid_ask, highest_bid);

            Depth { bids, asks }
        })
    }
}
