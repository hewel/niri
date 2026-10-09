#!/usr/bin/env bash
# Build only the explicitly selected integration and adjacent Smithay revision.
set -euo pipefail
niri_source=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
source "$niri_source/docs/personal/versions.env"
smithay_source="$niri_source/../smithay"
if [[ ! -d "$smithay_source" ]]; then
    printf 'Missing adjacent Smithay checkout: %s\n' "$smithay_source" >&2
    exit 1
fi
actual_smithay=$(git -C "$smithay_source" rev-parse HEAD)
if [[ "$actual_smithay" != "$SMITHAY_REV" ]]; then
    printf 'Smithay mismatch: expected %s, got %s\n' "$SMITHAY_REV" "$actual_smithay" >&2
    exit 1
fi
if ! git -C "$smithay_source" diff --quiet HEAD; then
    printf 'Smithay has tracked changes; the recorded revision would not describe this build.\n' >&2
    exit 1
fi
if ! git -C "$niri_source" merge-base --is-ancestor "$NIRI_SPICY_BASE" HEAD; then
    printf 'The recorded spicy base is not an ancestor of this integration.\n' >&2
    exit 1
fi
cd -- "$niri_source"
if ! git diff --quiet HEAD; then
    printf 'niri has tracked changes; commit the intended personal patches before a reproducible build.\n' >&2
    exit 1
fi
printf 'niri %s\nSmithay %s\n' "$(git rev-parse HEAD)" "$SMITHAY_REV"
NIRI_BUILD_COMMIT=$(git rev-parse HEAD) cargo build --release --locked --package niri --target-dir "$niri_source/target"
./target/release/niri validate -c examples/personal/glass.kdl
./target/release/niri validate -c examples/personal/glass-test-preset.kdl
./target/release/niri validate -c examples/personal/hdr-dp-1.kdl
./target/release/niri validate -c examples/personal/smooth-corners.kdl
./target/release/niri validate -c examples/personal/smooth-hdr-dp-1.kdl
