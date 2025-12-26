#!/bin/bash
# Helper script to run Vortex commands as root

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🔧 Vortex Root Helper${NC}\n"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: This script must be run as root${NC}"
    echo "Usage: sudo ./scripts/run-as-root.sh"
    exit 1
fi

# Find cargo
CARGO_BIN=""
if command -v cargo &> /dev/null; then
    CARGO_BIN="cargo"
elif [ -f "$HOME/.cargo/bin/cargo" ]; then
    CARGO_BIN="$HOME/.cargo/bin/cargo"
elif [ -f "/root/.cargo/bin/cargo" ]; then
    CARGO_BIN="/root/.cargo/bin/cargo"
else
    echo -e "${RED}Error: cargo not found${NC}"
    echo "Please install Rust: https://rustup.rs/"
    exit 1
fi

echo -e "${GREEN}Using cargo at: $CARGO_BIN${NC}\n"

# Menu
echo "Select an option:"
echo "  1) Build release binary"
echo "  2) Run namespace demo"
echo "  3) Run container (interactive)"
echo "  4) Run tests"
echo "  5) Exit"
echo

read -p "Enter choice [1-5]: " choice

case $choice in
    1)
        echo -e "\n${YELLOW}Building release binary...${NC}"
        $CARGO_BIN build --release --bin vortex
        echo -e "${GREEN}✅ Binary built: target/release/vortex${NC}"
        ;;
    2)
        echo -e "\n${YELLOW}Running namespace demo...${NC}"
        $CARGO_BIN run --example namespace_demo -p vortex-namespace
        ;;
    3)
        echo -e "\n${YELLOW}Running container...${NC}"
        read -p "Container ID: " cid
        read -p "Command (default: /bin/echo Hello): " cmd
        cmd=${cmd:-/bin/echo Hello}

        $CARGO_BIN build --release --bin vortex
        ./target/release/vortex run --id "$cid" -- $cmd
        ;;
    4)
        echo -e "\n${YELLOW}Running tests...${NC}"
        $CARGO_BIN test -p vortex-namespace -- --ignored --nocapture
        ;;
    5)
        echo "Goodbye!"
        exit 0
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac