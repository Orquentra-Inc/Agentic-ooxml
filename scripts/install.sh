#!/bin/sh
# Install the latest Agentic OOXML release binaries (agentic-ooxml, agentic-ooxml-mcp).
#
#   curl -fsSL https://raw.githubusercontent.com/Orquentra-Inc/Agentic-ooxml/main/scripts/install.sh | sh
#
# Downloads the archive for this OS and CPU from GitHub Releases, verifies it against the release's
# SHA256SUMS, and installs into ~/.local/bin. This needs a published release; until one exists, build
# from source instead (see the README).
#
# Environment:
#   AO_VERSION      release tag to install, for example v0.1.0 (default: the latest release)
#   AO_INSTALL_DIR  install directory (default: $HOME/.local/bin)
#   AO_BASE_URL     base URL holding the assets (default: the GitHub release download URL); for testing
#
# The checksum file comes from the same place as the archive, so this detects corruption, not a
# compromised release. The binaries are not signed.
set -eu

REPO="Orquentra-Inc/Agentic-ooxml"
INSTALL_DIR="${AO_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*"; }
die() { printf 'install.sh: %s\n' "$*" >&2; exit 1; }

need() { command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"; }
need curl
need tar
need uname
need mktemp

os="$(uname -s)"
arch="$(uname -m)"
case "$os/$arch" in
  Linux/x86_64) target="x86_64-unknown-linux-gnu" ;;
  Darwin/arm64 | Darwin/aarch64) target="aarch64-apple-darwin" ;;
  Darwin/x86_64) target="x86_64-apple-darwin" ;;
  *) die "no prebuilt binary for $os/$arch. Build from source: cargo install --git https://github.com/$REPO agentic-ooxml-cli agentic-ooxml-mcp" ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
  sha256() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum >/dev/null 2>&1; then
  sha256() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
  die "need sha256sum or shasum to verify the download"
fi

version="${AO_VERSION:-}"
if [ -z "$version" ]; then
  say "Looking up the latest release of $REPO"
  version="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" |
    sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)" ||
    die "could not query the latest release (is there one yet?)"
  [ -n "$version" ] || die "could not determine the latest release tag (is there one yet?)"
fi

name="agentic-ooxml-$version-$target"
asset="$name.tar.gz"
base="${AO_BASE_URL:-https://github.com/$REPO/releases/download/$version}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

say "Downloading $asset"
curl -fsSL -o "$tmp/$asset" "$base/$asset" || die "download failed: $base/$asset"
curl -fsSL -o "$tmp/SHA256SUMS" "$base/SHA256SUMS" || die "download failed: $base/SHA256SUMS"

expected="$(awk -v f="$asset" '$2 == f || $2 == "*" f { print $1; exit }' "$tmp/SHA256SUMS")"
[ -n "$expected" ] || die "$asset is not listed in SHA256SUMS"
actual="$(sha256 "$tmp/$asset")"
[ "$expected" = "$actual" ] || die "checksum mismatch for $asset (expected $expected, got $actual)"
say "Checksum OK"

tar -xzf "$tmp/$asset" -C "$tmp"
mkdir -p "$INSTALL_DIR"
for bin in agentic-ooxml agentic-ooxml-mcp; do
  [ -f "$tmp/$name/$bin" ] || die "$bin not found in the archive"
  cp "$tmp/$name/$bin" "$INSTALL_DIR/$bin.new"
  chmod 755 "$INSTALL_DIR/$bin.new"
  mv -f "$INSTALL_DIR/$bin.new" "$INSTALL_DIR/$bin"
done

say "Installed agentic-ooxml and agentic-ooxml-mcp $version into $INSTALL_DIR"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) say "Add $INSTALL_DIR to your PATH, for example: export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac
