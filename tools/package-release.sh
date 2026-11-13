#!/usr/bin/env bash
# Cross-compiles the Windows build from Linux and packages it:
#   dist/BounceRE/                              ready to run, or to load with
#                                               RecompLauncher's "Add a build"
#   dist/BounceRE-v<version>-Windows-x64.zip    for GitHub Releases
# Both hold Bounce.exe, recomplauncher.json, How to play.txt and LICENSE.
# No original game files are included: players bring their own JAR.
# Needs: rustup target x86_64-pc-windows-gnu, mingw64-gcc, cmake, ninja, python3.
# SDL3 is built from source and linked statically, so the .exe has no DLLs.
set -euo pipefail
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cargo build --release --target x86_64-pc-windows-gnu
target=${CARGO_TARGET_DIR:-target}
stage=dist/BounceRE
mkdir -p "$stage"
cp "$target/x86_64-pc-windows-gnu/release/bounce.exe" "$stage/Bounce.exe"
cp recomplauncher.json LICENSE "$stage/"
cat > "$stage/How to play.txt" <<TXT
Bounce RE $version

This needs your own copy of the original game: Bounce.jar
(Nokia, 128x128, version 1.9). It isn't included.

1. Put this folder somewhere you can write to (not Program Files).
2. Run Bounce.exe and drop the JAR on the window, or press
   Fire / A to pick it. You can also put the JAR next to the exe.

Move: arrows / WASD / D-pad. Jump: Up, Space or A. Menu: Esc or B.

Saves: save\   Settings: settings.toml   Logs: logs\\
More: https://github.com/Xarishark/bounce-re
TXT
zip="dist/BounceRE-v$version-Windows-x64.zip"
rm -f "$zip"
# only the release files: nothing a local run may have created (save/, data/, logs/, settings.toml)
python3 - "$stage" "$zip" <<'PY'
import os, sys, zipfile
stage, out = sys.argv[1], sys.argv[2]
keep = {"Bounce.exe", "recomplauncher.json", "LICENSE", "How to play.txt"}
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    for name in sorted(os.listdir(stage)):
        if name in keep:
            z.write(os.path.join(stage, name), name)
PY
echo "$stage/ and $zip:"
python3 -c "import zipfile,sys; [print(' ', i.filename, i.file_size) for i in zipfile.ZipFile(sys.argv[1]).infolist()]" "$zip"
