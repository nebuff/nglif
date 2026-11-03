#!/bin/bash
# Smoke test for a built nglif.app, as CI runs it on Apple silicon and Intel machines:
#
#   util/macos/ci-smoke.sh target/macos/nglif-<version>-macos-universal.dmg   (or nglif.app)
#
# 1. installs the app from the disk image the way a user would (drag out of the .dmg);
# 2. opens, saves and flattens a glyph headless;
# 3. opens examples/Q_.glif through LaunchServices, as a Finder double-click does, on the
#    machine's own display and then on a virtual Retina display, takes a screenshot of the
#    window each time and checks the glyph is drawn in the middle of it.
#
# Screenshots and logs land in smoke/ (or $SMOKE_OUT). This opens windows: run it on a CI
# machine, not on a desktop you're using.
set -euo pipefail

cd "$(dirname "$0")/../.."
ROOT=$PWD
input=${1:?usage: util/macos/ci-smoke.sh <nglif.dmg | nglif.app>}
out=${SMOKE_OUT:-$ROOT/smoke}
rm -rf "$out"
mkdir -p "$out/work" "$out/tools"

fail=0
check() { if eval "$2"; then printf '  ok    %s\n' "$1"; else printf '  FAIL  %s\n' "$1"; fail=1; fi; }
say() { printf '\033[1m==> %s\033[0m\n' "$*"; }

say "Tools"
swiftc -O util/macos/ci/inspect.swift -o "$out/tools/inspect" 2>/dev/null
clang -fobjc-arc -framework Foundation -framework CoreGraphics util/macos/ci/virtual-display.m -o "$out/tools/virtual-display"
inspect=$out/tools/inspect

say "Installing from $input"
if [[ $input == *.dmg ]]; then
    mnt=$(mktemp -d)
    hdiutil attach -quiet -nobrowse -readonly -mountpoint "$mnt" "$input"
    cp -R "$mnt/nglif.app" "$out/"
    check "disk image has an Applications link" "[ -L '$mnt/Applications' ]"
    hdiutil detach -quiet "$mnt"
    app=$out/nglif.app
else
    app=$input
fi
bin=$app/Contents/MacOS/nglif
native=$(uname -m)
check "signature verifies" "codesign --verify --strict '$app'"
check "universal binary ($(lipo -archs "$bin"))" "lipo '$bin' -verify_arch arm64 x86_64"
echo "  this machine: $native, macOS $(sw_vers -productVersion)"

say "Headless ($native)"
cp examples/Q_.glif "$out/work/"
(cd "$out/work" && "$ROOT/$bin" --save Q_.glif && rm Q_.glif && "$ROOT/$bin" --flatten Q_.glifjson) \
    >"$out/headless.log" 2>&1 || true
check "opens, saves and flattens examples/Q_.glif" "grep -q '<contour>' '$out/work/Q_.glif' 2>/dev/null"

# Opens a copy of Q_.glif the way Finder does, screenshots the window, closes the app.
gui() {
    local name=$1 want_scale=$2
    cp examples/Q_.glif "$out/work/$name.glif"
    open -n -a "$app" --env RUST_LOG=info,nglif=debug --env NO_WRITE_DEFAULT_KEYBINDS=1 \
        --stdout "$out/$name.out" --stderr "$out/$name.log" "$out/work/$name.glif"
    local id x y w h scale
    if ! read -r id x y w h scale < <("$inspect" window xyz.nebuff.nglif 60); then
        check "$name: window opened" false
        return
    fi
    if [ "${scale%.*}" != "$want_scale" ]; then
        # Where the window opens is up to macOS. Intel CI machines accept the virtual display as
        # main but still open new windows on their own 1x screen; nglif can't be tested at 2x there.
        echo "  skip  $name: macOS opened the window on a ${scale}x screen, not the ${want_scale}x one"
        pkill -9 -x nglif || true
        return
    fi
    sleep 3 # a few frames
    screencapture -l"$id" -o -x "$out/$name.png" || true
    pkill -x nglif || true # SDL turns SIGTERM into a quit request
    sleep 1
    pkill -9 -x nglif || true
    local pw ox oy
    pw=$(sips -g pixelWidth "$out/$name.png" 2>/dev/null | awk '/pixelWidth/{print $2}')
    read -r ox oy < <("$inspect" origin "$out/$name.png") || true
    echo "  $name: window ${w}x${h} points at ${scale}x, screenshot ${pw:-?} px wide, glyph origin at ${ox:-?} of the width"
    grep -h 'Window is' "$out/$name.log" | sed 's/^.*\] /  log: /'
    check "$name: got the document from LaunchServices, not argv" "grep -q 'macOS asked us to open' '$out/$name.log'"
    check "$name: display is ${want_scale}x" "[ '${scale%.*}' = '$want_scale' ]"
    check "$name: nglif knows it is ${want_scale} px/pt" "grep -q '(${want_scale} px/pt)' '$out/$name.log'"
    check "$name: window drawn at full resolution" "[ '${pw:-0}' -ge $((w * want_scale - 2)) ]"
    check "$name: glyph origin in the middle of the canvas" "awk 'BEGIN{exit !(${ox:-0} > 0.4 && ${ox:-0} < 0.6)}'"
    check "$name: no errors logged" "! grep -q -e 'panicked' -e ' ERROR nglif' '$out/$name.log'"
}

say "GUI on this machine's display"
gui standard "$("$inspect" scale)"

say "GUI on a virtual Retina display"
"$out/tools/virtual-display" 1440 900 >"$out/virtual-display.log" 2>&1 &
vd=$!
sleep 3
cat "$out/virtual-display.log"
if grep -q 'main=yes' "$out/virtual-display.log"; then
    gui retina 2
else
    echo "  skip  could not make a virtual Retina display the main one here"
fi
kill "$vd" 2>/dev/null || true

[ $fail = 0 ] || { echo "smoke test FAILED (logs and screenshots in $out)"; exit 1; }
echo "smoke test passed (screenshots in $out)"
