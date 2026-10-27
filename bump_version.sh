#!/usr/bin/env bash
# Usage: ./bump_version.sh <new-version>
# Example: ./bump_version.sh 0.5.0
#
# Bumps the workspace version, which ratslang, ratslang-cli and ratslang-lsp
# share, the AUR packages and the VS Code extension.

set -euo pipefail

NEW_VERSION="${1:-}"
if [[ -z "$NEW_VERSION" ]]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 0.5.0"
    exit 1
fi

if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9._-]+)?(\+[a-zA-Z0-9._-]+)?$ ]]; then
    echo "Error: version must be semver format (e.g. 1.2.3 or 1.2.3-alpha.1)"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

CARGO_TOML="Cargo.toml"

DETECTED_VERSION="$(
    awk -F'"' '
        /^\[workspace\.package\]/ { in_package = 1; next }
        /^\[/ { in_package = 0 }
        in_package && /^version = "/ { print $2; exit }
    ' "$CARGO_TOML"
)"

if [[ -z "$DETECTED_VERSION" ]]; then
    echo "Error: could not detect the workspace version from $CARGO_TOML"
    exit 1
fi

echo "Bumping the workspace: $DETECTED_VERSION -> $NEW_VERSION"

# Portable sed -i
sed_i() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i "" "$@"
    else
        sed -i "$@"
    fi
}

# The workspace version, which ratslang and ratslang-cli inherit
awk -v new_version="$NEW_VERSION" '
    /^\[workspace\.package\]/ { in_package = 1; print; next }
    /^\[/ { in_package = 0 }
    in_package && /^version = "/ {
        print "version = \"" new_version "\""
        next
    }
    { print }
' "$CARGO_TOML" > "$CARGO_TOML.tmp"
mv "$CARGO_TOML.tmp" "$CARGO_TOML"

# The ratslang dependency the CLI and the LSP inherit
sed_i -E "s/^(ratslang = \{ version = \")[^\"]+/\1${NEW_VERSION}/" "$CARGO_TOML"

# The AUR packages start a new release at pkgrel 1
for pkgbuild in packaging/arch/PKGBUILD packaging/arch-bin/PKGBUILD; do
    sed_i -e "s/^pkgver=.*/pkgver=${NEW_VERSION}/" -e "s/^pkgrel=.*/pkgrel=1/" "$pkgbuild"
done

# The VS Code extension shares the release version
sed_i -E "s/^(  \"version\": \")[^\"]+/\1${NEW_VERSION}/" vscode/package.json
sed_i -E "/^  \"name\": \"ratslang\"|^      \"name\": \"ratslang\"/{n;s/(\"version\": \")[^\"]+/\1${NEW_VERSION}/;}" vscode/package-lock.json

# The versions named in the docs and READMEs
echo ""
echo "── Syncing versions in the docs ──────────────────────────────────────"
python3 scripts/check_versions.py --fix

echo ""
echo "── Updating Cargo.lock ───────────────────────────────────────────────"
cargo update --workspace

echo ""
echo "── git diff ──────────────────────────────────────────────────────────"
git diff -- "$CARGO_TOML" Cargo.lock packaging vscode/package.json vscode/package-lock.json README.md docs/docs
