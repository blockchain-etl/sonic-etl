#!/bin/bash

# Get the absolute path of the script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Define paths
SOURCE_DIR="$SCRIPT_DIR/../schemas"
DEST_DIR="$SCRIPT_DIR/../extractor_transformer/src/sonic_config/schemas"
BUILD_DIR="$SCRIPT_DIR/../extractor_transformer"

# Create the destination directory if it doesn't exist
mkdir -p "$DEST_DIR"

# Copy the schemas directory and its contents
cp -r "$SOURCE_DIR"/* "$DEST_DIR/"

echo "Schemas copied successfully to $DEST_DIR"

# Check if Cargo is installed, install it if necessary
if ! command -v cargo &>/dev/null; then
    echo "Cargo is not installed. Install Rust and Cargo"
    exit 1
else
    echo "Cargo already installed."
fi

# Navigate to the build directory and run cargo build without warnings
if [ -d "$BUILD_DIR" ]; then
    cd "$BUILD_DIR" || exit 1
    echo "Building extractor_transformer..."
    RUSTFLAGS="-A warnings" cargo build
    echo "Complete"
    exit 0
else
    echo "Error: Directory $BUILD_DIR does not exist."
    exit 1
fi
