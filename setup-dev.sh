# Development environment setup script
#!/bin/bash

set -e

echo "🚀 Setting up GoQuant Oracle Development Environment"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running on macOS
if [[ "$OSTYPE" != "darwin"* ]]; then
    print_warning "This script is optimized for macOS. Some commands may need adjustment for other systems."
fi

# Check prerequisites
print_status "Checking prerequisites..."

# Check if Homebrew is installed
if ! command -v brew &> /dev/null; then
    print_error "Homebrew is required but not installed. Please install it first:"
    echo "  /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
    exit 1
fi

# Install Rust if not present
if ! command -v rustc &> /dev/null; then
    print_status "Installing Rust 1.75..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.75.0
    source $HOME/.cargo/env
else
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    print_success "Rust is already installed (version $RUST_VERSION)"
    
    # Update to 1.75 if older version
    if [[ "$RUST_VERSION" < "1.75.0" ]]; then
        print_status "Updating Rust to 1.75..."
        rustup update stable
        rustup default 1.75.0
    fi
fi

# Install Solana CLI
if ! command -v solana &> /dev/null; then
    print_status "Installing Solana CLI..."
    sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
else
    print_success "Solana CLI is already installed"
fi

# Install Anchor
if ! command -v anchor &> /dev/null; then
    print_status "Installing Anchor framework..."
    cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
    avm install 0.29.0
    avm use 0.29.0
else
    print_success "Anchor is already installed"
fi

# Install Node.js and Yarn (for Anchor tests)
if ! command -v node &> /dev/null; then
    print_status "Installing Node.js..."
    brew install node
else
    print_success "Node.js is already installed"
fi

if ! command -v yarn &> /dev/null; then
    print_status "Installing Yarn..."
    npm install -g yarn
else
    print_success "Yarn is already installed"
fi

# Install PostgreSQL
if ! command -v psql &> /dev/null; then
    print_status "Installing PostgreSQL..."
    brew install postgresql@15
    brew services start postgresql@15
else
    print_success "PostgreSQL is already installed"
fi

# Install Redis
if ! command -v redis-cli &> /dev/null; then
    print_status "Installing Redis..."
    brew install redis
    brew services start redis
else
    print_success "Redis is already installed"
fi

# Install Docker
if ! command -v docker &> /dev/null; then
    print_warning "Docker is not installed. Please install Docker Desktop from https://docker.com"
else
    print_success "Docker is already installed"
fi

# Install additional Rust tools
print_status "Installing additional Rust tools..."
cargo install cargo-watch sqlx-cli --features postgres

# Setup Solana keypair if it doesn't exist
if [ ! -f "$HOME/.config/solana/id.json" ]; then
    print_status "Creating Solana keypair..."
    mkdir -p "$HOME/.config/solana"
    solana-keygen new --outfile "$HOME/.config/solana/id.json" --no-bip39-passphrase
else
    print_success "Solana keypair already exists"
fi

# Set Solana config to localnet for development
print_status "Setting Solana to localnet..."
solana config set --url localhost

print_success "✅ Development environment setup complete!"
echo ""
echo "Next steps:"
echo "1. Start the infrastructure: docker-compose up -d postgres redis"
echo "2. Build the Anchor program: cd programs/oracle-integration && anchor build"
echo "3. Start the backend: cd backend && cargo run"
echo ""
echo "Useful commands:"
echo "  - Check Solana config: solana config get"
echo "  - Start local validator: solana-test-validator"
echo "  - Watch backend logs: cd backend && cargo watch -x run"
echo "  - Run tests: cd programs/oracle-integration && anchor test"
