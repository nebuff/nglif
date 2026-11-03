#!/bin/bash
# Builds nglif.app for macOS: one universal binary (Apple silicon + Intel), an icon, Finder
# document types for .glif/.glifjson, an ad-hoc signature, and optionally a .dmg. Then it
# checks what it built, so a broken bundle fails here instead of on someone else's Mac.
#
#   util/macos/build-app.sh            build target/macos/nglif.app
#   util/macos/build-app.sh --dmg      ... and target/macos/nglif-<version>-macos-universal.dmg
#   util/macos/build-app.sh --arch arm64|x86_64   one architecture only (quicker, for testing)
#
# Needs the Xcode command line tools (xcode-select --install), rustup and CMake
# (brew install cmake). rustup fetches the nightly pinned in rust-toolchain by itself.
#
# Set CODESIGN_IDENTITY to sign with a "Developer ID Application" certificate instead of the
# default ad-hoc signature; the app is then ready for `xcrun notarytool submit`.
set -euo pipefail

cd "$(dirname "$0")/../.."
ROOT=$PWD

want_dmg=0
archs="arm64 x86_64"
while [ $# -gt 0 ]; do
    case "$1" in
        --dmg) want_dmg=1 ;;
        --arch) shift; archs="${1:?--arch needs arm64 or x86_64}" ;;
        -h|--help) sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown option: $1 (try --help)" >&2; exit 2 ;;
    esac
    shift
done

say() { printf '\033[1m==> %s\033[0m\n' "$*"; }
die() { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }

[ "$(uname -s)" = Darwin ] || die "this script builds the macOS app and only runs on macOS"
command -v cmake >/dev/null || die "CMake is needed to build SDL2: brew install cmake (or https://cmake.org/download/)"
command -v rustup >/dev/null || die "rustup is needed: https://rustup.rs"

version=$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
toolchain=$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain)
[ -n "$version" ] && [ -n "$toolchain" ] || die "could not read the version or toolchain"

triple() { case "$1" in arm64) echo aarch64-apple-darwin ;; x86_64) echo x86_64-apple-darwin ;; *) die "unknown arch $1" ;; esac; }
min_macos() { case "$1" in arm64) echo 11.0 ;; x86_64) echo 10.15 ;; esac; }

# A Homebrew `rust` earlier in PATH would shadow rustup's proxies and build with the wrong
# compiler (no nightly features, no cross-architecture std), so put the pinned toolchain first.
say "Rust $toolchain (from rust-toolchain)"
targets=""
for a in $archs; do targets="$targets --target $(triple "$a")"; done
# shellcheck disable=SC2086
rustup toolchain install "$toolchain" --profile minimal $targets >/dev/null
export PATH="$(dirname "$(rustup which --toolchain "$toolchain" rustc)"):$PATH"

binaries=""
for a in $archs; do
    t=$(triple "$a")
    say "Building $a ($t, macOS $(min_macos "$a")+)"
    MACOSX_DEPLOYMENT_TARGET=$(min_macos "$a") cargo build --release --locked --target "$t"
    binaries="$binaries target/$t/release/nglif"
done

OUT=target/macos
APP=$OUT/nglif.app
say "Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
# shellcheck disable=SC2086
lipo -create -output "$APP/Contents/MacOS/nglif" $binaries
sed "s/@VERSION@/$version/g" util/macos/Info.plist > "$APP/Contents/Info.plist"
printf 'APPL????' > "$APP/Contents/PkgInfo"
cp resources/macos/nglif.icns "$APP/Contents/Resources/"
cp LICENSE AUTHORS "$APP/Contents/Resources/"
xattr -cr "$APP"

identity=${CODESIGN_IDENTITY:--}
if [ "$identity" = - ]; then
    say "Signing ad hoc (Apple silicon refuses to run unsigned code)"
    codesign --force --sign - "$APP"
else
    say "Signing as $identity, hardened runtime"
    codesign --force --sign "$identity" --options runtime --timestamp "$APP"
fi

say "Checking the bundle"
bin=$APP/Contents/MacOS/nglif
fail=0
check() { if eval "$2"; then printf '  ok    %s\n' "$1"; else printf '  FAIL  %s\n' "$1"; fail=1; fi; }
check "Info.plist is valid" "plutil -lint -s '$APP/Contents/Info.plist'"
check "signature verifies" "codesign --verify --strict '$APP' 2>/dev/null"
for a in $archs; do
    check "$a slice present" "lipo '$bin' -verify_arch $a"
    # Anything outside the OS (a Homebrew SDL2, libpng, ...) would be missing on other Macs.
    foreign=$(otool -arch "$a" -L "$bin" | tail -n +2 | awk '{print $1}' | grep -v -e '^/System/Library/' -e '^/usr/lib/' || true)
    check "$a links only macOS system libraries${foreign:+ (not: $foreign)}" "[ -z '$foreign' ]"
    minos=$(otool -arch "$a" -l "$bin" | awk '/LC_BUILD_VERSION/{f=1} f&&/minos/{print $2; exit}')
    check "$a runs on macOS $(min_macos "$a") and later (minos $minos)" "[ '$minos' = '$(min_macos "$a")' ]"
done

# Open, save and flatten a glyph with each slice, headless (no window), and compare the files.
# On an Apple silicon Mac, Rosetta 2 runs the Intel slice; an Intel Mac can't run arm64.
smoke=$(mktemp -d)
for a in $archs; do
    if ! arch "-$a" /usr/bin/true 2>/dev/null; then
        printf '  skip  %s smoke test (this Mac cannot run %s code%s)\n' "$a" "$a" \
            "$([ "$a" = x86_64 ] && echo '; softwareupdate --install-rosetta')"
        continue
    fi
    mkdir -p "$smoke/$a"
    cp examples/Q_.glif "$smoke/$a/"
    # --save writes Q_.glifjson; --flatten turns that back into a fresh Q_.glif.
    ( cd "$smoke/$a" && RUST_LOG=error arch "-$a" "$ROOT/$bin" --save Q_.glif \
        && rm Q_.glif && RUST_LOG=error arch "-$a" "$ROOT/$bin" --flatten Q_.glifjson ) >"$smoke/$a.log" 2>&1 || true
    check "$a opens, saves and flattens examples/Q_.glif" "[ -s '$smoke/$a/Q_.glifjson' ] && grep -q '<contour>' '$smoke/$a/Q_.glif' 2>/dev/null"
done
if [ -d "$smoke/arm64" ] && [ -d "$smoke/x86_64" ]; then
    check "arm64 and x86_64 write identical files" "diff -r '$smoke/arm64' '$smoke/x86_64' >/dev/null"
fi
rm -rf "$smoke"
[ $fail = 0 ] || die "the bundle failed its checks"

if [ $want_dmg = 1 ]; then
    suffix=$( [ "$archs" = "arm64 x86_64" ] && echo universal || echo "$archs" )
    dmg=$OUT/nglif-$version-macos-$suffix.dmg
    say "Making $dmg"
    stage=$(mktemp -d)
    cp -R "$APP" "$stage/"
    ln -s /Applications "$stage/Applications"
    cp util/macos/dmg-readme.txt "$stage/Read Me First.txt"
    cp LICENSE "$stage/LICENSE.txt"
    rm -f "$dmg"
    hdiutil create -quiet -volname "nglif $version" -srcfolder "$stage" -fs HFS+ -format UDZO -ov "$dmg"
    rm -rf "$stage"
    hdiutil verify -quiet "$dmg"
    printf '  ok    %s (%s)\n' "$dmg" "$(du -h "$dmg" | cut -f1)"
fi

say "Done: $APP ($(lipo -archs "$bin"), nglif $version)"
