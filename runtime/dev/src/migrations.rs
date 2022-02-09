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

/// 1. SystemToDualRefCount: from `unique`  to `dual` reference counting.
/// 2. frame_system::Pallet<System>(automatically): from `dual` to dual `triple` reference counting.
pub struct SystemToDualRefCount;
impl frame_support::traits::OnRuntimeUpgrade for SystemToDualRefCount {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        frame_system::migrations::migrate_to_dual_ref_count::<Runtime>()
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

pub struct GrandpaStoragePrefixMigration;
impl frame_support::traits::OnRuntimeUpgrade for GrandpaStoragePrefixMigration {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        use frame_support::traits::PalletInfo;
        let name = <Runtime as frame_system::Config>::PalletInfo::name::<Grandpa>()
            .expect("grandpa is part of pallets in construct_runtime, so it has a name; qed");
        pallet_grandpa::migrations::v4::migrate::<Runtime, &str>(name)
    }
}

const COUNCIL_OLD_PREFIX: &str = "Instance1Collective";
/// Migrate from `Instance1Collective` to the new pallet prefix `Council`
pub struct CouncilStoragePrefixMigration;
impl frame_support::traits::OnRuntimeUpgrade for CouncilStoragePrefixMigration {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        pallet_collective::migrations::v4::migrate::<Runtime, Council, _>(COUNCIL_OLD_PREFIX)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        pallet_collective::migrations::v4::pre_migrate::<Council, _>(COUNCIL_OLD_PREFIX);
        Ok(())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        pallet_collective::migrations::v4::post_migrate::<Council, _>(COUNCIL_OLD_PREFIX);
        Ok(())
    }
}

const TECHNICAL_COMMITTEE_OLD_PREFIX: &str = "Instance2Collective";
/// Migrate from `Instance2Collective` to the new pallet prefix `TechnicalCommittee`
pub struct TechnicalCommitteeStoragePrefixMigration;
impl frame_support::traits::OnRuntimeUpgrade for TechnicalCommitteeStoragePrefixMigration {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        pallet_collective::migrations::v4::migrate::<Runtime, TechnicalCommittee, _>(
            TECHNICAL_COMMITTEE_OLD_PREFIX,
        )
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        pallet_collective::migrations::v4::pre_migrate::<TechnicalCommittee, _>(
            TECHNICAL_COMMITTEE_OLD_PREFIX,
        );
        Ok(())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        pallet_collective::migrations::v4::post_migrate::<TechnicalCommittee, _>(
            TECHNICAL_COMMITTEE_OLD_PREFIX,
        );
        Ok(())
    }
}

const TECHNICAL_MEMBERSHIP_OLD_PREFIX: &str = "Instance1Membership";
/// Migrate from `Instance1Membership` to the new pallet prefix `TechnicalMembership`
pub struct TechnicalMembershipStoragePrefixMigration;
impl frame_support::traits::OnRuntimeUpgrade for TechnicalMembershipStoragePrefixMigration {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        use frame_support::traits::PalletInfo;
        let name = <Runtime as frame_system::Config>::PalletInfo::name::<TechnicalMembership>()
            .expect("TechnicalMembership is part of runtime, so it has a name; qed");
        pallet_membership::migrations::v4::migrate::<Runtime, TechnicalMembership, _>(
            TECHNICAL_MEMBERSHIP_OLD_PREFIX,
            name,
        )
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        use frame_support::traits::PalletInfo;
        let name = <Runtime as frame_system::Config>::PalletInfo::name::<TechnicalMembership>()
            .expect("TechnicalMembership is part of runtime, so it has a name; qed");
        pallet_membership::migrations::v4::pre_migrate::<TechnicalMembership, _>(
            TECHNICAL_MEMBERSHIP_OLD_PREFIX,
            name,
        );
        Ok(())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        use frame_support::traits::PalletInfo;
        let name = <Runtime as frame_system::Config>::PalletInfo::name::<TechnicalMembership>()
            .expect("TechnicalMembership is part of runtime, so it has a name; qed");
        pallet_membership::migrations::v4::post_migrate::<TechnicalMembership, _>(
            TECHNICAL_MEMBERSHIP_OLD_PREFIX,
            name,
        );
        Ok(())
    }
}


