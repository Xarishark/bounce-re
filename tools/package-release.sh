#!/usr/bin/env bash
# Builds and packages a release for one system:
#   tools/package-release.sh windows   cross-compiles from Linux (MinGW) -> BounceRE-v<ver>-Windows-x64.zip
#   tools/package-release.sh linux     native build                      -> BounceRE-v<ver>-Linux-x64.tar.gz
#   tools/package-release.sh macos     native build on a Mac             -> BounceRE-v<ver>-macOS-<arch>.zip
# Each archive holds the program, recomplauncher.json (top level, for
# RecompLauncher), How to play.txt and LICENSE; dist/BounceRE/ is the same
# folder unpacked. SDL3 is built from source and linked statically, so
# nothing else needs installing. No original game files are included:
# players bring their own JAR.
# Needs: cmake, ninja or make, python3; for windows also rustup target
# x86_64-pc-windows-gnu and mingw64-gcc; for linux the X11/Wayland/audio
# development headers (see .github/workflows/build.yml).
set -euo pipefail
cd "$(dirname "$0")/.."
os=${1:-windows}
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
target=${CARGO_TARGET_DIR:-target}
stage=dist/BounceRE
# never wipe the stage: it may be a played folder with save/, data/ and settings.toml
rm -f "$stage/Bounce.exe" "$stage/bounce"
mkdir -p "$stage"
case "$os" in
    windows)
        cargo build --release --locked --target x86_64-pc-windows-gnu
        cp "$target/x86_64-pc-windows-gnu/release/bounce.exe" "$stage/Bounce.exe"
        program=Bounce.exe
        archive="dist/BounceRE-v$version-Windows-x64.zip"
        ;;
    linux | macos)
        cargo build --release --locked
        cp "$target/release/bounce" "$stage/bounce"
        program=bounce
        if [ "$os" = linux ]; then
            archive="dist/BounceRE-v$version-Linux-x64.tar.gz"
        else
            arch=$(uname -m)
            [ "$arch" = x86_64 ] && arch=x64
            archive="dist/BounceRE-v$version-macOS-$arch.zip"
        fi
        ;;
    *)
        echo "usage: $0 windows|linux|macos" >&2
        exit 2
        ;;
esac
cp recomplauncher.json LICENSE "$stage/"
mac_note=""
[ "$os" = macos ] && mac_note="
The build isn't signed: the first time, right-click bounce and choose Open,
or run: xattr -d com.apple.quarantine bounce"
cat > "$stage/How to play.txt" <<TXT
Bounce RE $version

This needs your own copy of the original game: Bounce.jar
(Nokia, 128x128, version 1.9). It isn't included.

1. Put this folder somewhere you can write to.
2. Run $program and drop the JAR on the window, or press
   Fire / A to pick it. You can also put the JAR next to the program.
$mac_note
Move: arrows / WASD / D-pad. Jump: Up, Space or A. Menu: Esc or B.
Full screen: F or F11.

Saves: save/   Settings: settings.toml   Logs: logs/
More: https://github.com/Xarishark/bounce-re
TXT
rm -f "$archive"
# only the release files: nothing a local run may have created (save/, data/, logs/, settings.toml)
python3 - "$stage" "$archive" "$program" <<'PY'
import os, sys, tarfile, zipfile
stage, out, program = sys.argv[1:4]
keep = [program, "recomplauncher.json", "LICENSE", "How to play.txt"]
if out.endswith(".tar.gz"):
    with tarfile.open(out, "w:gz") as t:
        for name in keep:
            t.add(os.path.join(stage, name), name)
else:
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for name in keep:
            path = os.path.join(stage, name)
            info = zipfile.ZipInfo.from_file(path, name)  # keeps the executable bit
            info.compress_type = zipfile.ZIP_DEFLATED
            with open(path, "rb") as f:
                z.writestr(info, f.read())
PY
echo "$stage/ and $archive"
