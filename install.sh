#!/bin/bash
# Install vortex CLI to system

set -e  # Exit on error

echo "🦀 Vortex Container Runtime - Installation Script"
echo ""

# Check if running from correct directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Must run from vortex-runtime root directory"
    exit 1
fi

# Build release version
echo "🔨 Building vortex-cli (release mode)..."
cargo build --release --package vortex-cli

# Check if build succeeded
if [ ! -f "target/release/vortex-cli" ]; then
    echo "❌ Error: Build failed - binary not found"
    exit 1
fi

echo "✅ Build complete"
echo ""

# Install to system
echo "📦 Installing to /usr/local/bin/vortex..."
sudo cp target/release/vortex-cli /usr/local/bin/vortex

# Verify installation
if [ -f "/usr/local/bin/vortex" ]; then
    echo "✅ Installation successful!"
    echo ""
    echo "📊 Binary info:"
    ls -lh /usr/local/bin/vortex
    echo ""
    echo "🚀 Usage:"
    echo "  sudo vortex run --id my-container"
    echo "  sudo vortex run --id app --cpu 1.0 --memory 512"
    echo "  sudo vortex --help"
    echo ""
    echo "🎉 Ready to use! Try: sudo vortex version"
else
    echo "❌ Error: Installation failed"
    exit 1
fi
