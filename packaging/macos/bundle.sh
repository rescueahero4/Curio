#!/usr/bin/env bash
#
# Assemble Curio.app and a drag-to-install .dmg (R-DEL-8).
#
# Usage, from the repo root:
#
#   packaging/macos/bundle.sh <version> <arm64-dir> <x86_64-dir> <out-dir>
#
# where the two directories each hold a `curio` and a `curio-nmh` built for that arch. CI
# passes the release target dirs; locally you can point both at the same directory to get a
# single-arch bundle for testing.
#
# ## On signing
#
# R-DEL-8 wants a Developer ID signature and a notarisation ticket. Both require a paid
# Apple Developer account, so this script signs properly when the credentials are present
# and falls back to an **ad-hoc** signature when they are not.
#
# The ad-hoc signature is not cosmetic and not optional: on Apple Silicon the kernel refuses
# to execute an arm64 binary carrying no signature at all, and `lipo` strips whatever
# signature its inputs had. Skipping this step produces a bundle that dies with SIGKILL on
# every Mac shipped since 2020. It does not satisfy Gatekeeper — users still take the
# Privacy & Security detour documented in the README — but it makes the app runnable.

set -euo pipefail

VERSION="${1:?version required}"
ARM_DIR="${2:?arm64 binary directory required}"
X86_DIR="${3:?x86_64 binary directory required}"
OUT_DIR="${4:?output directory required}"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"

APP="$OUT_DIR/Curio.app"
MACOS_DIR="$APP/Contents/MacOS"
RES_DIR="$APP/Contents/Resources"
# No version in the name, so the landing page can link /releases/latest/download/ once and
# never revisit it (D36). $VERSION is still required — it is what the Info.plist below is
# stamped with, which is where macOS reads the version from.
DMG="$OUT_DIR/curio-macos-universal.dmg"

rm -rf "$APP" "$DMG"
mkdir -p "$MACOS_DIR" "$RES_DIR"

# --- binaries ---------------------------------------------------------------------------
# Universal rather than two downloads. The whole app is a few MB; making a user identify
# their own CPU to pick a file is a worse trade than the extra megabytes (R-DEL-4 governs
# the binary's size, not the bundle's arch count).
for bin in curio curio-nmh; do
  lipo -create -output "$MACOS_DIR/$bin" "$ARM_DIR/$bin" "$X86_DIR/$bin"
  chmod +x "$MACOS_DIR/$bin"
done
lipo -info "$MACOS_DIR/curio"

# --- Info.plist -------------------------------------------------------------------------
sed "s/__VERSION__/$VERSION/g" "$HERE/Info.plist" > "$APP/Contents/Info.plist"
printf 'APPL????' > "$APP/Contents/PkgInfo"

# --- icon -------------------------------------------------------------------------------
# iconutil accepts a partial iconset and emits an icns containing whatever it was given, so
# missing large sizes degrade to a blurry Finder icon rather than a failed build.
ICONSET="$(mktemp -d)/curio.iconset"
mkdir -p "$ICONSET"
copy_icon() { # <source-px> <iconset-name>
  local src="$REPO/assets/brand/curio-mark-$1.png"
  [ -f "$src" ] && cp "$src" "$ICONSET/$2" || true
}
copy_icon 16  "icon_16x16.png"
copy_icon 32  "icon_16x16@2x.png"
copy_icon 32  "icon_32x32.png"
copy_icon 128 "icon_128x128.png"
copy_icon 256 "icon_128x128@2x.png"
copy_icon 256 "icon_256x256.png"
copy_icon 512 "icon_512x512.png"

if iconutil -c icns "$ICONSET" -o "$RES_DIR/curio.icns" 2>/dev/null; then
  echo "icon: built from $(ls "$ICONSET" | wc -l | tr -d ' ') sizes"
else
  echo "icon: iconutil failed; shipping without one" >&2
fi

# --- signature --------------------------------------------------------------------------
# Deepest-first: the nested helper must be sealed before the bundle that contains it, or the
# outer signature is invalidated the moment the inner one is written.
if [ -n "${MACOS_SIGN_IDENTITY:-}" ]; then
  echo "signing with Developer ID: $MACOS_SIGN_IDENTITY"
  codesign --force --options runtime --timestamp \
    --sign "$MACOS_SIGN_IDENTITY" "$MACOS_DIR/curio-nmh"
  codesign --force --options runtime --timestamp \
    --sign "$MACOS_SIGN_IDENTITY" "$APP"
  codesign --verify --deep --strict --verbose=2 "$APP"
else
  echo "no MACOS_SIGN_IDENTITY: ad-hoc signing so the bundle can execute on Apple Silicon"
  codesign --force --sign - "$MACOS_DIR/curio-nmh"
  codesign --force --sign - "$APP"
fi

# --- dmg --------------------------------------------------------------------------------
STAGE="$(mktemp -d)/Curio"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"

hdiutil create \
  -volname "Curio" \
  -srcfolder "$STAGE" \
  -fs HFS+ \
  -format UDZO \
  -ov \
  "$DMG"

# --- notarisation -----------------------------------------------------------------------
# Only reachable with a real Developer ID signature; an ad-hoc bundle is rejected by the
# service, so there is nothing to attempt.
if [ -n "${MACOS_SIGN_IDENTITY:-}" ] && [ -n "${AC_API_KEY_PATH:-}" ]; then
  echo "submitting for notarisation..."
  xcrun notarytool submit "$DMG" \
    --key "$AC_API_KEY_PATH" \
    --key-id "$AC_API_KEY_ID" \
    --issuer "$AC_API_ISSUER" \
    --wait
  xcrun stapler staple "$DMG"
  xcrun stapler validate "$DMG"
else
  echo "not notarised — users will need the Privacy & Security detour (see README)"
fi

echo "built $DMG"
