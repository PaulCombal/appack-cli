#!/usr/bin/env bash

if ! command -v snap &> /dev/null; then
    echo "Error: 'snap' command not found. Please install snapd first."
    exit 1
fi

# Extract the version number from Cargo.toml
RAW_CARGO_URL="https://raw.githubusercontent.com/PaulCombal/appack-cli/master/Cargo.toml"
echo "Fetching version info..."
python3 -c "import urllib.request; urllib.request.urlretrieve('$RAW_CARGO_URL', '/tmp/Cargo.toml')"
VERSION=$(grep -m 1 '^version =' /tmp/Cargo.toml | cut -d '"' -f 2)

if [ -z "$VERSION" ]; then
    echo "Error: Could not extract version."
    exit 1
fi

echo "Detected version: $VERSION"

# Download the snap
URL="https://github.com/PaulCombal/appack-cli/releases/download/v${VERSION}/appack_${VERSION}_amd64.snap"
OUTPUT="/tmp/appack_${VERSION}_amd64.snap"
echo "Downloading $URL..."
python3 -c "import urllib.request; urllib.request.urlretrieve('$URL', '$OUTPUT')"

# Install and Connect
if [ -f "$OUTPUT" ]; then
    pkexec sh <<EOF
snap install "$OUTPUT" --dangerous
snap connect appack:kvm
snap connect appack:dot-local-share-applications
EOF
    rm "$OUTPUT"
else
    echo "Download failed: File not found."
    exit 1
fi

echo "Installation complete. Rerun this script periodically to check for updates"