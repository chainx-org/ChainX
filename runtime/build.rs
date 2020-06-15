use wasm_builder_runner::WasmBuilder;

fn main() {
    WasmBuilder::new()
        .with_current_project()
        .with_wasm_builder_from_git(
            "https://github.com/paritytech/substrate.git",
            "34695a85650b58bcd7d7e2a677cafc2921251d68",
        )
        .export_heap_base()
        .import_memory()
        .build()
}
