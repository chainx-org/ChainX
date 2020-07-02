use wasm_builder_runner::WasmBuilder;

fn main() {
    WasmBuilder::new()
        .with_current_project()
        .with_wasm_builder_from_git(
            "https://github.com/paritytech/substrate.git",
            "00768a1f21a579c478fe5d4f51e1fa71f7db9fd4",
        )
        .export_heap_base()
        .import_memory()
        .build()
}
