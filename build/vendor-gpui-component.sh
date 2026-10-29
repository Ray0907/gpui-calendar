#!/bin/sh
# Rebuilds vendor/gpui-component: the crates.io 0.8.1 crate plus patches/gpui-component-0.8.1-calendar-cursor.patch. Run once per fresh clone, before cargo build.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
patch_file=$root/patches/gpui-component-0.8.1-calendar-cursor.patch
src=
for d in "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/gpui-component-0.8.1; do [ -d "$d" ] && src=$d && break; done
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
if [ -z "$src" ]; then
  # Fresh clone: the registry has no copy yet (cargo cannot fetch while the patch path is missing), so download it and check the pinned sha256 (the patched Cargo.lock no longer carries it).
  curl -fsSL https://static.crates.io/crates/gpui-component/gpui-component-0.8.1.crate -o "$tmp/c.crate"
  want=46ce53430332e30f303364eaabfa10ed2aa23dfaf2a147e04370e0e9c423578b
  [ "$(shasum -a 256 "$tmp/c.crate" | cut -d' ' -f1)" = "$want" ] || { echo "downloaded crate checksum differs from the pinned sha256" >&2; exit 1; }
  tar -xzf "$tmp/c.crate" -C "$tmp"
  src=$tmp/gpui-component-0.8.1
fi
grep -q '^version = "0.8.1"' "$src/Cargo.toml" || { echo "registry crate is not gpui-component 0.8.1" >&2; exit 1; }
cp -R "$src" "$tmp/crate"
# cargo writes this file only for published crates; a path dependency does not need it and it names the upstream commit
rm -f "$tmp/crate/.cargo_vcs_info.json"
(cd "$tmp/crate" && patch -p1 --forward --silent < "$patch_file") || { echo "patch does not apply to the registry crate" >&2; exit 1; }
rm -rf "$root/vendor/gpui-component"
mkdir -p "$root/vendor"
mv "$tmp/crate" "$root/vendor/gpui-component"
echo "vendored gpui-component 0.8.1 + calendar cursor patch"
