use crate::{GenesisConfig, Trait};

mod balances {
    // TODO
    // Set PCX free balance.
}

mod xassets {
    // TODO
    // Set XBTC free balance.
}

mod xmining_asset {
    // TODO
    // Set the cross mining record
    //  - Set the weight related to zero.
    //  - Set the XBTC amount.
}

mod xstaking {
    // TODO
    // Simulate the bond operation.
}

pub(crate) fn initialize<T: Trait>(config: &GenesisConfig<T>) {
    let now = std::time::Instant::now();

    /*
    balances::initialize::<T>(
        &config.params.balances,
        config.root_endowed,
        config.initial_authorities_endowed,
    );
    xassets::initialize::<T>(&config.params.xassets);
    xstaking::initialize::<T>(&config.params.xstaking);
    xmining_asset::initialize::<T>(&config.params.xmining_asset);
    */

    xp_logging::info!(
        "Took {:?}ms to orchestrate the exported state from ChainX 1.0",
        now.elapsed().as_millis()
    );
}
