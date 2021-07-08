ifeq ($(origin FEDORA_RELEASE), undefined)
else
  FEDORA_RELEASE_ARGS = --manifest-path=${MANIFEST_PATH}
endif

ifeq ($(origin MANIFEST_PATH), undefined)
else
  MANIFEST_PATH_ARGS = --manifest-path=${MANIFEST_PATH}
endif

RUST_2018_IDIOMS = -D bare-trait-objects  \
                   -D ellipsis-inclusive-range-patterns \
                   -D unused-extern-crates

DENY = -D warnings -D future-incompatible -D unused ${RUST_2018_IDIOMS}

# Clippy-related lints
CLIPPY_CARGO = -D clippy::cargo_common_metadata \
               -D clippy::multiple_crate_versions \
               -D clippy::wildcard_dependencies

# Clippy allow/deny adjudications for pedantic lints
#
# Allows represent lints we fail but which we may
# conclude are helpful at some time.
CLIPPY_PEDANTIC = -A clippy::upper_case_acronyms

${HOME}/.cargo/bin/cargo-tree:
	cargo install cargo-tree

${HOME}/.cargo/bin/cargo-audit:
	cargo install cargo-audit

tree: ${HOME}/.cargo/bin/cargo-tree
	PATH=${HOME}/.cargo/bin:${PATH} cargo tree

audit: ${HOME}/.cargo/bin/cargo-audit
	PATH=${HOME}/.cargo/bin:${PATH} cargo audit -D warnings

SET_LOWER_BOUNDS ?=
test-set-lower-bounds:
	echo "Testing that SET_LOWER_BOUNDS environment variable is set to a valid path"
	test -e "${SET_LOWER_BOUNDS}"

verify-dependency-bounds: test-set-lower-bounds
	RUSTFLAGS="${DENY}" cargo build ${MANIFEST_PATH_ARGS}
	${SET_LOWER_BOUNDS} ${MANIFEST_PATH_ARGS}
	RUSTFLAGS="${DENY}" cargo build ${MANIFEST_PATH_ARGS}

test-compare-fedora-versions:
	echo "Testing that COMPARE_FEDORA_VERSIONS environment variable is set to a valid path"
	test -e "${COMPARE_FEDORA_VERSIONS}"

check-fedora-versions: test-compare-fedora-versions
	${COMPARE_FEDORA_VERSIONS} ${MANIFEST_PATH_ARGS} ${FEDORA_RELEASE_ARGS}

fmt:
	cargo fmt

travis_fmt:
	cargo fmt -- --check

build:
	RUSTFLAGS="${DENY}" cargo build

build-tests:
	RUSTFLAGS="${DENY}" cargo test --no-run

test:
	RUSTFLAGS="${DENY}" RUST_BACKTRACE=1 cargo test -- --skip sudo_ --skip loop_

sudo_test:
	RUSTFLAGS="${DENY}" RUST_BACKTRACE=1 RUST_TEST_THREADS=1 cargo test

clippy:
	RUSTFLAGS="${DENY}" cargo clippy --all-targets --all-features -- -D clippy::needless_borrow ${CLIPPY_CARGO} ${CLIPPY_PEDANTIC}

docs:
	cargo doc --no-deps

yamllint:
	yamllint --strict .github/workflows/*.yml

.PHONY:
	audit
	build
	check-fedora-versions:
	clippy
	docs
	fmt
	sudo_test
	test
	test-compare-fedora-versions
	test-set-lower-bounds
	travis_fmt
	tree
	verify-dependency-bounds
	yamllint
