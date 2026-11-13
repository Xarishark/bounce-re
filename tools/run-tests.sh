#!/usr/bin/env bash
# Runs every headless test script against a release build.
# Needs your own Bounce.jar: game-files/ by default, or set
# BOUNCE_JAR. Each script runs in a fresh temporary game folder
# (BOUNCE_HOME), so nothing touches a real save. Screenshots go to
# <target dir>/test-shots/<script>/.
set -uo pipefail
cd "$(dirname "$0")/.."
jar=${BOUNCE_JAR:-game-files/Bounce.jar}
if [ ! -f "$jar" ]; then
    echo "The tests need your copy of the game: put Bounce.jar in game-files/ or set BOUNCE_JAR."
    exit 2
fi
cargo build --release || exit 1
target=${CARGO_TARGET_DIR:-target}
bin=$target/release/bounce
[ -x "$bin" ] || bin=$target/release/bounce.exe

# prepare the game data once, then copy it into each test's folder
prepared=$(mktemp -d)
if ! BOUNCE_HOME="$prepared" "$bin" --prepare --game "$jar" >/dev/null; then
    echo "Couldn't prepare the game data from $jar"
    exit 1
fi
fail=0
for script in tests/scripts/*.txt; do
    name=$(basename "$script" .txt)
    out=$target/test-shots/$name
    home=$(mktemp -d)
    cp -r "$prepared/data" "$home/"
    log=$(mktemp)
    rm -rf "$out"
    echo "== $name"
    BOUNCE_HOME="$home" "$bin" --headless --script "$script" --out "$out" >"$log" 2>&1
    status=$?
    grep -E '^(pos|stat)' "$log" | sed 's/^/   /'
    if [ "$status" -ne 0 ]; then
        echo "   FAILED (exit $status)"
        sed 's/^/   | /' "$log" | tail -5
        fail=1
    else
        echo "   ok, $(ls "$out" 2>/dev/null | wc -l) screenshots in $out"
    fi
    rm -rf "$home" "$log"
done
rm -rf "$prepared"
exit $fail
