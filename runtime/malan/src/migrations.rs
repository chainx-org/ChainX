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

// pub struct SystemToTripleRefCount;
// impl frame_support::traits::OnRuntimeUpgrade for SystemToTripleRefCount {
//     fn on_runtime_upgrade() -> frame_support::weights::Weight {
//         frame_system::migrations::migrate_from_unique_to_triple_ref_count::<Runtime>()
//     }
// }

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

const TIPS_OLD_PREFIX: &str = "Treasury";
/// Migrate pallet-tips from `Treasury` to the new pallet prefix `Tips`
pub struct MigrateTipsPalletPrefix;
impl frame_support::traits::OnRuntimeUpgrade for MigrateTipsPalletPrefix {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        pallet_tips::migrations::v4::migrate::<Runtime, Tips, _>(TIPS_OLD_PREFIX)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        pallet_tips::migrations::v4::pre_migrate::<Runtime, Tips, _>(TIPS_OLD_PREFIX);
        Ok(())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        pallet_tips::migrations::v4::post_migrate::<Runtime, Tips, _>(TIPS_OLD_PREFIX);
        Ok(())
    }
}

const BOUNTIES_OLD_PREFIX: &str = "Treasury";
/// Migrate from 'Treasury' to the new prefix 'Bounties'
pub struct BountiesPrefixMigration;
impl frame_support::traits::OnRuntimeUpgrade for BountiesPrefixMigration {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        use frame_support::traits::PalletInfo;
        let name = <Runtime as frame_system::Config>::PalletInfo::name::<Bounties>()
            .expect("Bounties is part of runtime, so it has a name; qed");
        pallet_bounties::migrations::v4::migrate::<Runtime, Bounties, _>(BOUNTIES_OLD_PREFIX, name)
    }
    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        use frame_support::traits::PalletInfo;
        let name = <Runtime as frame_system::Config>::PalletInfo::name::<Bounties>()
            .expect("Bounties is part of runtime, so it has a name; qed");
        pallet_bounties::migrations::v4::pre_migration::<Runtime, Bounties, _>(
            BOUNTIES_OLD_PREFIX,
            name,
        );
        Ok(())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        use frame_support::traits::PalletInfo;
        let name = <Runtime as frame_system::Config>::PalletInfo::name::<Bounties>()
            .expect("Bounties is part of runtime, so it has a name; qed");
        pallet_bounties::migrations::v4::post_migration::<Runtime, Bounties, _>(
            BOUNTIES_OLD_PREFIX,
            name,
        );
        Ok(())
    }
}

/// Migrate from 'PhragmenElection' to the new prefix 'Elections'
pub struct ElectionsPrefixMigration;
impl frame_support::traits::OnRuntimeUpgrade for ElectionsPrefixMigration {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        use frame_support::traits::PalletInfo;
        let name = <Runtime as frame_system::Config>::PalletInfo::name::<Elections>()
            .expect("Elections is part of runtime, so it has a name; qed");
        pallet_elections_phragmen::migrations::v4::migrate::<Runtime, _>(name)
    }
}

pub struct XGatewayCommonStorageMigration;
impl frame_support::traits::OnRuntimeUpgrade for XGatewayCommonStorageMigration {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        xpallet_gateway_common::migrations::taproot::apply::<Runtime>()
    }
}

use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
pub struct CustomOnRuntimeUpgrades;
impl OnRuntimeUpgrade for CustomOnRuntimeUpgrades {
    fn on_runtime_upgrade() -> Weight {
        let mut weight = 0;

        // // 1. RemoveCollectiveFlip
        // frame_support::log::info!("🔍️ RemoveCollectiveFlip start");
        // weight += <RemoveCollectiveFlip as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 RemoveCollectiveFlip end");

        // // 2. MigratePalletVersionToStorageVersion
        // frame_support::log::info!("🔍️ MigratePalletVersionToStorageVersion start");
        // weight += <MigratePalletVersionToStorageVersion as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 MigratePalletVersionToStorageVersion end");

        // // 3. PhragmenElectionDepositRuntimeUpgrade
        // frame_support::log::info!("🔍️ PhragmenElectionDepositRuntimeUpgrade start");
        // frame_support::traits::StorageVersion::new(0).put::<Elections>();
        // weight += <PhragmenElectionDepositRuntimeUpgrade as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 PhragmenElectionDepositRuntimeUpgrade end");

        // // 4. SystemToTripleRefCount
        // frame_support::log::info!("🔍️ SystemToTripleRefCount start");
        // weight += <SystemToTripleRefCount as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 SystemToTripleRefCount end");

        // // 5. BabeEpochConfigMigrations
        // frame_support::log::info!("🔍️ BabeEpochConfigMigrations start");
        // weight += <BabeEpochConfigMigrations as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 BabeEpochConfigMigrations end");

        // // 6. GrandpaStoragePrefixMigration
        // frame_support::log::info!("🔍️ GrandpaStoragePrefixMigration start");
        // frame_support::traits::StorageVersion::new(0).put::<Grandpa>();
        // weight += <GrandpaStoragePrefixMigration as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 GrandpaStoragePrefixMigration end");

        // // 7. CouncilStoragePrefixMigration
        // frame_support::log::info!("🔍️ CouncilStoragePrefixMigration start");
        // frame_support::traits::StorageVersion::new(0).put::<Council>();
        // weight += <CouncilStoragePrefixMigration as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 CouncilStoragePrefixMigration end");

        // // 8. TechnicalCommitteeStoragePrefixMigration
        // frame_support::log::info!("🔍️ TechnicalCommitteeStoragePrefixMigration start");
        // frame_support::traits::StorageVersion::new(0).put::<TechnicalCommittee>();
        // weight +=
        //     <TechnicalCommitteeStoragePrefixMigration as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 TechnicalCommitteeStoragePrefixMigration end");

        // // 9. TechnicalMembershipStoragePrefixMigration
        // frame_support::log::info!("🔍️ TechnicalMembershipStoragePrefixMigration start");
        // frame_support::traits::StorageVersion::new(0).put::<TechnicalMembership>();
        // weight +=
        //     <TechnicalMembershipStoragePrefixMigration as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 TechnicalMembershipStoragePrefixMigration end");

        // // 10. CouncilStoragePrefixMigration
        // frame_support::log::info!("🔍️ CouncilStoragePrefixMigration start");
        // frame_support::traits::StorageVersion::new(0).put::<Council>();
        // weight += <CouncilStoragePrefixMigration as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 CouncilStoragePrefixMigration end");

        // // 11. MigrateTipsPalletPrefix
        // frame_support::log::info!("🔍️ MigrateTipsPalletPrefix start");
        // frame_support::traits::StorageVersion::new(0).put::<Tips>();
        // weight += <MigrateTipsPalletPrefix as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 MigrateTipsPalletPrefix end");

        // // 12. BountiesPrefixMigration
        // frame_support::log::info!("🔍️ BountiesPrefixMigration start");
        // frame_support::traits::StorageVersion::new(0).put::<Bounties>();
        // weight += <BountiesPrefixMigration as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 BountiesPrefixMigration end");

        // // 13. ElectionsPrefixMigration
        // frame_support::log::info!("🔍️ ElectionsPrefixMigration start");
        // weight += <ElectionsPrefixMigration as OnRuntimeUpgrade>::on_runtime_upgrade();
        // frame_support::log::info!("🚀 ElectionsPrefixMigration end");

        // 14. XGatewayCommonStorageMigration
        frame_support::log::info!("🔍️ XGatewayCommonStorageMigration start");
        weight += <XGatewayCommonStorageMigration as OnRuntimeUpgrade>::on_runtime_upgrade();
        frame_support::log::info!("🚀 XGatewayCommonStorageMigration end");

        weight
    }
}
