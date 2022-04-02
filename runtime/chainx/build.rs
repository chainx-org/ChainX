// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

use substrate_wasm_builder::WasmBuilder;

fn main() {
    WasmBuilder::new()
        .with_current_project()
        .import_memory()
        .export_heap_base()
        .build()
}
