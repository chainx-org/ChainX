use super::*;

pub struct RemoveCollectiveFlip;
impl frame_support::traits::OnRuntimeUpgrade for RemoveCollectiveFlip {
    fn on_runtime_upgrade() -> Weight {
        use frame_support::storage::migration;
        // Remove the storage value `RandomMaterial` from removed pallet `RandomnessCollectiveFlip`
        migration::remove_storage_prefix(b"RandomnessCollectiveFlip", b"RandomMaterial", b"");
        <Runtime as frame_system::Config>::DbWeight::get().writes(1)
    }
}

/// Migrate from `PalletVersion` to the new `StorageVersion`
pub struct MigratePalletVersionToStorageVersion;
impl frame_support::traits::OnRuntimeUpgrade for MigratePalletVersionToStorageVersion {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        frame_support::migrations::migrate_from_pallet_version_to_storage_version::<
            AllPalletsWithSystem,
        >(&RocksDbWeight::get())
    }
}

// 10 PCX
const OLD_CANDIDACY_BOND: Balance = 1000 * DOLLARS;
// 10 mPCX
const OLD_VOTING_BOND: Balance = DOLLARS;
pub struct PhragmenElectionDepositRuntimeUpgrade;
impl pallet_elections_phragmen::migrations::v3::V2ToV3 for PhragmenElectionDepositRuntimeUpgrade {
    type Pallet = Elections;
    type AccountId = AccountId;
    type Balance = Balance;
}
impl frame_support::traits::OnRuntimeUpgrade for PhragmenElectionDepositRuntimeUpgrade {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        pallet_elections_phragmen::migrations::v3::apply::<Self>(
            OLD_VOTING_BOND,
            OLD_CANDIDACY_BOND,
        )
    }
}

impl pallet_babe::migrations::BabePalletPrefix for Runtime {
    fn pallet_prefix() -> &'static str {
        "Babe"
    }
}

pub struct BabeEpochConfigMigrations;
impl frame_support::traits::OnRuntimeUpgrade for BabeEpochConfigMigrations {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        pallet_babe::migrations::add_epoch_configuration::<Runtime>(
            sp_consensus_babe::BabeEpochConfiguration {
                allowed_slots: PrimaryAndSecondaryPlainSlots,
                ..BABE_GENESIS_EPOCH_CONFIG
            },
        )
    }
}
