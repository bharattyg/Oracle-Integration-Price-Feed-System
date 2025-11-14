#!/bin/bash

# GoQuant Oracle System - Dependency Verification
# Run this script to verify all dependencies are properly installed

set -e

echo "🔍 GoQuant Oracle System - Dependency Verification"
echo "=================================================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

check_command() {
    if command -v "$1" &> /dev/null; then
        echo -e "✅ ${GREEN}$1${NC} - $($1 --version | head -n1)"
        return 0
    else
        echo -e "❌ ${RED}$1 not found${NC}"
        return 1
    fi
}

check_service() {
    if brew services list | grep -q "$1.*started"; then
        echo -e "✅ ${GREEN}$1 service${NC} - Running"
        return 0
    else
        echo -e "⚠️  ${YELLOW}$1 service${NC} - Not running (can be started with: brew services start $1)"
        return 1
    fi
}

echo ""
echo "🛠️  Core Development Tools:"
check_command "rustc"
check_command "cargo"
check_command "solana"
check_command "anchor"

echo ""
echo "🗄️  Database & Cache:"
check_command "psql"
check_command "redis-cli"

echo ""
echo "🐳 Container Tools:"
check_command "docker"
check_command "docker-compose"

echo ""
echo "📦 Additional Tools:"
if command -v cargo-watch &> /dev/null; then
    echo -e "✅ ${GREEN}cargo-watch${NC} - Available"
elif command -v watchexec &> /dev/null; then
    echo -e "✅ ${GREEN}watchexec${NC} - Available (alternative to cargo-watch)"
else
    echo -e "⚠️  ${YELLOW}No file watcher found${NC} - Install with: cargo install cargo-watch"
fi

check_command "sqlx"

echo ""
echo "🚀 Services Status:"
if command -v brew &> /dev/null; then
    check_service "postgresql@14"
    check_service "redis"
else
    echo "⚠️  Homebrew not available - cannot check service status"
fi

echo ""
echo "📁 Project Structure:"
if [ -f "./programs/oracle-integration/src/lib.rs" ]; then
    echo -e "✅ ${GREEN}Solana Program${NC} - Oracle integration contract ready"
else
    echo -e "❌ ${RED}Solana Program${NC} - Missing oracle integration contract"
fi

if [ -f "./backend/src/main.rs" ]; then
    echo -e "✅ ${GREEN}Backend Service${NC} - Rust backend ready"
else
    echo -e "❌ ${RED}Backend Service${NC} - Missing backend main.rs"
fi

if [ -f "./db/schema.sql" ]; then
    echo -e "✅ ${GREEN}Database Schema${NC} - PostgreSQL schema ready"
else
    echo -e "❌ ${RED}Database Schema${NC} - Missing schema.sql"
fi

if [ -f "./docker-compose.yml" ] && [ -f "./docker-compose.dev.yml" ]; then
    echo -e "✅ ${GREEN}Docker Configuration${NC} - Production & development configs ready"
else
    echo -e "❌ ${RED}Docker Configuration${NC} - Missing docker-compose files"
fi

if [ -f "./Makefile" ]; then
    echo -e "✅ ${GREEN}Build System${NC} - Makefile ready"
else
    echo -e "❌ ${RED}Build System${NC} - Missing Makefile"
fi

echo ""
echo "🎯 Next Steps:"
echo "1. Start services: make dev"
echo "2. Build Anchor program: cd programs/oracle-integration && anchor build"
echo "3. Build backend: cd backend && cargo build"
echo "4. Run tests: make test"
echo ""
echo "📖 Available Commands:"
echo "  make help     - Show all available commands"
echo "  make setup    - Run initial setup"
echo "  make dev      - Start development environment"
echo "  make build    - Build all components"
echo "  make test     - Run all tests"
echo ""

# Check if we're in the right directory
if [ ! -f "./Makefile" ]; then
    echo -e "⚠️  ${YELLOW}Note: Run this script from the project root directory${NC}"
fi

echo "✨ Verification complete!"
