use wasm_builder_runner::WasmBuilder;

fn main() {
    WasmBuilder::new()
        .with_current_project()
        .with_wasm_builder_from_git(
            "https://github.com/paritytech/substrate.git",
            "45b9f0a9cbf901abaa9f1fca5fe8baeed029133d",
        )
        .export_heap_base()
        .import_memory()
        .build()
}
