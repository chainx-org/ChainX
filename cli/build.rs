// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

use substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};

fn main() {
    generate_cargo_keys();

    rerun_if_git_head_changed();
}
