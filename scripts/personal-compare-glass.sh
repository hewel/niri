#!/usr/bin/env bash
# Explicit, opt-in SDR appearance comparison. No installation or session switch.
set -euo pipefail
comparison_source=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
source "$comparison_source/docs/personal/versions.env"
if [[ $# != 2 ]]; then
    printf 'Usage: %s <Niri-glass-checkout> <output-directory>\n' "$0" >&2
    exit 1
fi
reference_source=$(cd -- "$1" && pwd)
mkdir -p -- "$2"
comparison_output=$(cd -- "$2" && pwd)
git -C "$reference_source" cat-file -e "$GLASS_REV:src/render_helpers/shaders/clipped_surface.frag"
if [[ $(git -C "$comparison_source/../smithay" rev-parse HEAD) != "$SMITHAY_REV" ]] || ! git -C "$comparison_source/../smithay" diff --quiet HEAD; then
    printf 'Adjacent Smithay does not match the clean recorded revision.\n' >&2
    exit 1
fi
cd -- "$comparison_source"
NIRI_TEST_REQUIRE_GPU=1 LIBGL_ALWAYS_SOFTWARE=1 \
    NIRI_GLASS_REFERENCE_ROOT="$reference_source" NIRI_GLASS_COMPARE_DIR="$comparison_output" \
    cargo test --locked --lib compare_glass_appearance -- --ignored --nocapture
NIRI_TEST_REQUIRE_GPU=1 LIBGL_ALWAYS_SOFTWARE=1 python3 scripts/personal-glass-report.py "$comparison_output"
