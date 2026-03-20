#!/usr/bin/env bash
set -e

CRATE_SRC="crate/src"
CORE_DST="$CRATE_SRC/core"
CARGO_TOML="crate/Cargo.toml"
LIB_RS="$CRATE_SRC/lib.rs"

cleanup() {
    echo "→ cleaning up..."
    rm -rf "$CORE_DST"
    git checkout -- "$CARGO_TOML" "$LIB_RS" \
        "$CRATE_SRC/state.rs" "$CRATE_SRC/load.rs" \
        "$CRATE_SRC/manifest.rs" "$CRATE_SRC/store.rs"
    echo "→ done"
}
trap cleanup EXIT

echo "→ copying core/src/ to crate/src/core/"
cp -r core/src/. "$CORE_DST"

echo "→ patching crate/Cargo.toml (remove core dep, add exclude)"
sed -i '/^core = /d' "$CARGO_TOML"
sed -i '/^keywords = /i exclude = ["examples/*"]' "$CARGO_TOML"

echo "→ patching crate/src/lib.rs"
sed -i 's/^pub mod common;/mod core;\npub mod common;/' "$LIB_RS"

echo "→ patching use core:: → use crate::core::"
for f in "$CRATE_SRC/state.rs" "$CRATE_SRC/load.rs" "$CRATE_SRC/manifest.rs" "$CRATE_SRC/store.rs"; do
    sed -i 's/^use core::/use crate::core::/g' "$f"
done

echo "→ cargo publish $@"
cargo publish --manifest-path crate/Cargo.toml "$@"
