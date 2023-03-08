use chrono::{DateTime, NaiveDateTime, Local, FixedOffset};

pub const INITIAL_REWARD: u128 = 5_000_000_000;
pub const FIXED_TOTAL: u128 = 2_100_000_000_000_000;
// 7296763 0x568301d70fede34c3324ba0c4fa6833f61d072685e6dfd9a02542645a06b1822
// total_issuance 1464367470000000
// timestamp 1672817322000
fn main() {
    let mut total_issuance_7296763 = 1464367470000000u128;
    let current_reward = 2500000000u128;
    let mut sessions = 0;

    loop {
        let reward = this_session_reward(total_issuance_7296763);
        if reward < current_reward {
            break
        };

        total_issuance_7296763 += reward;
        sessions += 1;
    };

    let halving_timestamp_ms: i64 = 1672817322000 + (sessions * 50 * 6 * 1000);
    let naive_date_time = NaiveDateTime::from_timestamp_millis(halving_timestamp_ms).unwrap();
    let date_time = DateTime::<Local>::from_utc(naive_date_time, FixedOffset::east_opt(8 * 3600).unwrap());

    println!(
        "ChainX(PCX) Halving: #{}, at {} [{:?}]",
        7296763 + sessions * 50,
        halving_timestamp_ms,
        date_time
    )
}

fn this_session_reward(total_issuance: u128) -> u128 {
    let tt = (FIXED_TOTAL / (FIXED_TOTAL - total_issuance)) as f32;
    let halving_epoch = tt.log2().trunc() as u32; // n

    INITIAL_REWARD / ((1_u32 << halving_epoch) as u128)
}