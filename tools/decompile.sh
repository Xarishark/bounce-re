#!/usr/bin/env bash
# Decompiles the original JAR with Vineflower, for reference while working on
# the port. Put Bounce.jar in game-files/ (git-ignored).
# Output: game-files/decompiled/*.java. Classes whose names differ only in case
# (A.java vs a.java) would collide on Windows, so upper-case ones get a U_ prefix.
# Needs: java 17+, curl, unzip.
set -euo pipefail
cd "$(dirname "$0")/.."
jar=${1:-game-files/Bounce.jar}
vf=game-files/vineflower-1.11.1.jar
[ -f "$vf" ] || curl -sSL -o "$vf" https://repo1.maven.org/maven2/org/vineflower/vineflower/1.11.1/vineflower-1.11.1.jar
tmp=$(mktemp -d)
java -jar "$vf" -dgs=1 -rsy=1 -ind="    " "$jar" "$tmp/"
# Vineflower writes either .java files or a jar of them, depending on version
for j in "$tmp"/*.jar; do
    [ -f "$j" ] && (cd "$tmp" && unzip -oq "$j" && rm "$j")
done
mkdir -p game-files/decompiled
find "$tmp" -name '*.java' | while read -r f; do
    rel=${f#"$tmp/"}
    b=$(echo "$rel" | sed 's|/|_|g')
    case "$b" in [A-Z]*.java) b="U_$b" ;; esac
    cp "$f" "game-files/decompiled/$b"
done
rm -rf "$tmp"
echo "Decompiled to game-files/decompiled/"
