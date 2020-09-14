// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use wasm_builder_runner::WasmBuilder;

fn main() {
    WasmBuilder::new()
        .with_current_project()
        .with_wasm_builder_from_crates("2.0.0")
        .import_memory()
        .export_heap_base()
        .build()
}
