SHELL := /bin/bash
#ENABLE_FEATURES ?= default

default: release

.PHONY: all

all: format build test

pre-clippy: unset-override
	@rustup component add clippy-preview

clippy: pre-clippy
	@cargo clippy --release --all --all-targets -- \
		-A clippy::module_inception -A clippy::needless_pass_by_value \
		-A clippy::cognitive_complexity -A clippy::unreadable_literal \
		-A clippy::should_implement_trait -A clippy::verbose_bit_mask \
		-A clippy::implicit_hasher -A clippy::large_enum_variant \
		-A clippy::new_without_default -A clippy::blacklisted_name \
		-A clippy::neg_cmp_op_on_partial_ord -A clippy::too_many_arguments \
		-A clippy::excessive_precision -A clippy::collapsible_if \
		-D warnings

build:
	cargo build --release #--features "${ENABLE_FEATURES}"

release:
	@cargo build --release #--features "${ENABLE_FEATURES}"

test:
	export LOG_LEVEL=DEBUG && \
	export RUST_BACKTRACE=1 && \
	cargo test --release --all -- --nocapture

unset-override:
	@# unset first in case of any previous overrides
	@if rustup override list | grep `pwd` > /dev/null; then rustup override unset; fi

pre-format: unset-override
	@rustup component add rustfmt-preview

format: pre-format
	@cargo fmt --all -- --check >/dev/null || \
	cargo fmt --all

clean:
	@cargo clean
