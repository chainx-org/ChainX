// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! A set of constant values used in chainx runtime.

/// Money matters.
pub mod currency {
    use chainx_primitives::Balance;

    pub const PCXS: Balance = 100_000_000; // 8 decimals
    pub const DOLLARS: Balance = PCXS / 100; // 1000_000
    pub const CENTS: Balance = DOLLARS / 100; // 10_000
    pub const MILLICENTS: Balance = CENTS / 1_000; // 10

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 20 * DOLLARS + (bytes as Balance) * 100 * MILLICENTS
    }
}

/// Time.
pub mod time {
    use chainx_primitives::{BlockNumber, Moment};

    pub const MILLISECS_PER_BLOCK: Moment = 6000;
    pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;
    pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 7 * DAYS;

    // These time units are defined in number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;

    // 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
    pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}
