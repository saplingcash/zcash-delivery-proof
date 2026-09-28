#!/usr/bin/env bash
# Builds the WebAssembly package into wasm/pkg from pinned inputs only, so anyone can rebuild the committed
# files byte for byte:
#
#   - the compiler: wasm/rust-toolchain.toml (with the wasm32-unknown-unknown target);
#   - the crates: wasm/Cargo.lock (--locked);
#   - the C compiler: secp256k1's C library (a dependency of zcash_primitives, through zcash_script) is
#     compiled into the WebAssembly with `clang`, so its exact build is an input too: Ubuntu 26.04's clang
#     21.1.8 (6ubuntu1);
#   - the bindings: the official wasm-bindgen release binary at exactly the version of the wasm-bindgen
#     crate in wasm/Cargo.lock (wasm-bindgen-0.2.129-x86_64-unknown-linux-musl, checked by its SHA-256);
#     no wasm-opt pass;
#   - built from one fixed directory (Cargo hashes a path dependency's location into symbol names), with
#     every source and build path remapped, so the binary carries no path of the machine that built it.
#
#   ./wasm/build.sh           # build and copy into wasm/pkg
#   ./wasm/build.sh --check   # build and fail if it differs from the committed files (CI)
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/.." && pwd)"
want="$(awk '/^name = "wasm-bindgen"$/{getline; gsub(/version = |"/, ""); print}' "$here/Cargo.lock")"
bindgen="${WASM_BINDGEN:-wasm-bindgen}"
have="$("$bindgen" --version | awk '{print $2}')"
if [ "$have" != "$want" ]; then
  echo "wasm-bindgen $have found; this build needs exactly $want (the official release binary)" >&2
  exit 2
fi
bindgen_sha="fb59da714982a04273e9e4b09a85e523e5dd04de35b1a4a8b1641785ee7a71cf"
if [ "$(sha256sum < "$(command -v "$bindgen")" | cut -c1-64)" != "$bindgen_sha" ]; then
  echo "wasm-bindgen $have is not the official release binary (wasm-bindgen-$want-x86_64-unknown-linux-musl, SHA-256 $bindgen_sha)" >&2
  exit 2
fi
cc_want="Ubuntu clang version 21.1.8 (6ubuntu1)"
cc_have="$(clang --version 2>/dev/null | head -n 1 || true)"
if [ "$cc_have" != "$cc_want" ]; then
  echo "clang: \"$cc_have\" found; this build needs \"$cc_want\" (Ubuntu 26.04's clang package)" >&2
  exit 2
fi
src=/tmp/zcash-delivery-proof-wasm-src
target=/tmp/zcash-delivery-proof-wasm-target
rm -rf "$src" "$target"
mkdir -p "$src"
(cd "$repo" && tar --exclude=target --exclude=.git --exclude=wasm/pkg -cf - Cargo.toml src wasm/Cargo.toml wasm/Cargo.lock wasm/rust-toolchain.toml wasm/src) | tar -xf - -C "$src"
out="$(mktemp -d)"
export RUSTFLAGS="--remap-path-prefix=$src=/zdp --remap-path-prefix=$target=/target --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=${RUSTUP_HOME:-$HOME/.rustup}=/rustup"
(cd "$src/wasm" && CARGO_TARGET_DIR="$target" cargo build --release --locked --target wasm32-unknown-unknown)
"$bindgen" --target web --out-dir "$out" "$target/wasm32-unknown-unknown/release/zcash_delivery_proof_wasm.wasm"
files="zcash_delivery_proof_wasm.js zcash_delivery_proof_wasm.d.ts zcash_delivery_proof_wasm_bg.wasm"
if [ "${1:-}" = "--check" ]; then
  status=0
  for f in $files; do
    if ! cmp -s "$out/$f" "$here/pkg/$f"; then
      echo "$f differs from the committed file: built $(sha256sum < "$out/$f" | cut -c1-64), committed $(sha256sum < "$here/pkg/$f" 2>/dev/null | cut -c1-64)" >&2
      status=1
    fi
  done
  (cd "$out" && sha256sum zcash_delivery_proof_wasm_bg.wasm) | cmp -s - "$here/SHA256" || { echo "wasm/SHA256 is not the built file's hash" >&2; status=1; }
  [ $status = 0 ] && echo "the committed WebAssembly is what this source builds: $(sha256sum < "$out/zcash_delivery_proof_wasm_bg.wasm" | cut -c1-64)"
  exit $status
fi
mkdir -p "$here/pkg"
for f in $files; do cp "$out/$f" "$here/pkg/"; done
(cd "$out" && sha256sum zcash_delivery_proof_wasm_bg.wasm) > "$here/SHA256"
cat "$here/SHA256"
