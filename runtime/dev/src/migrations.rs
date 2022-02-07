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
