# Makefile for GoQuant Oracle System

.PHONY: help setup dev prod test clean logs

# Default target
help:
	@echo "GoQuant Oracle System - Available Commands:"
	@echo ""
	@echo "Development:"
	@echo "  make setup     - Setup development environment"
	@echo "  make dev       - Start development environment"
	@echo "  make test      - Run all tests"
	@echo "  make anchor    - Build and test Anchor program"
	@echo ""
	@echo "Production:"
	@echo "  make prod      - Start production environment"
	@echo "  make build     - Build all components"
	@echo ""
	@echo "Utilities:"
	@echo "  make logs      - View application logs"
	@echo "  make clean     - Clean up containers and volumes"
	@echo "  make db-reset  - Reset database"
	@echo "  make lint      - Run code linting"
	@echo ""

# Setup development environment
setup:
	@echo "🚀 Setting up development environment..."
	./setup-dev.sh

# Start development environment
dev:
	@echo "🔧 Starting development environment..."
	docker-compose -f docker-compose.yml -f docker-compose.dev.yml --profile development up -d
	@echo "✅ Development environment started!"
	@echo "Services available at:"
	@echo "  - API: http://localhost:3000"
	@echo "  - WebSocket: ws://localhost:3001"
	@echo "  - PostgreSQL: localhost:5432"
	@echo "  - Redis: localhost:6379"
	@echo "  - PgAdmin: http://localhost:5050"
	@echo "  - Grafana: http://localhost:3002"
	@echo "  - Solana RPC: http://localhost:8899"

# Start production environment
prod:
	@echo "🚀 Starting production environment..."
	docker-compose --profile production up -d

# Build all components
build:
	@echo "🔨 Building Anchor program..."
	cd programs/oracle-integration && anchor build
	@echo "🔨 Building backend..."
	cd backend && cargo build --release
	@echo "✅ Build complete!"

# Run tests
test:
	@echo "🧪 Running Rust tests..."
	cd backend && cargo test
	@echo "🧪 Running Anchor tests..."
	cd programs/oracle-integration && anchor test --skip-local-validator
	@echo "✅ All tests passed!"

# Build and test Anchor program
anchor:
	@echo "⚓ Building Anchor program..."
	cd programs/oracle-integration && anchor build
	@echo "⚓ Testing Anchor program..."
	cd programs/oracle-integration && anchor test
	@echo "✅ Anchor build and test complete!"

# View logs
logs:
	docker-compose logs -f oracle-backend

# Clean up
clean:
	@echo "🧹 Cleaning up..."
	docker-compose down -v
	docker-compose -f docker-compose.dev.yml down -v
	docker system prune -f
	@echo "✅ Cleanup complete!"

# Reset database
db-reset:
	@echo "🔄 Resetting database..."
	docker-compose exec postgres psql -U postgres -c "DROP DATABASE IF EXISTS goquant;"
	docker-compose exec postgres psql -U postgres -c "CREATE DATABASE goquant;"
	docker-compose exec postgres psql -U postgres -d goquant -f /docker-entrypoint-initdb.d/01-schema.sql
	@echo "✅ Database reset complete!"

# Lint code
lint:
	@echo "🔍 Linting Rust code..."
	cd backend && cargo clippy -- -D warnings
	cd programs/oracle-integration && cargo clippy -- -D warnings
	@echo "🔍 Formatting code..."
	cd backend && cargo fmt --check
	cd programs/oracle-integration && cargo fmt --check
	@echo "✅ Linting complete!"

# Watch backend in development
watch:
	cd backend && watchexec -e rs,toml -r cargo run

# Check system health
health:
	@echo "🏥 Checking system health..."
	@curl -s http://localhost:3000/health | jq . || echo "Backend not responding"
	@docker-compose ps

# Generate documentation
docs:
	@echo "📚 Generating documentation..."
	cd backend && cargo doc --no-deps --open
	cd programs/oracle-integration && anchor idl parse --file src/lib.rs

# Backup database
backup:
	@echo "💾 Creating database backup..."
	docker-compose exec postgres pg_dump -U postgres goquant > backup_$(shell date +%Y%m%d_%H%M%S).sql
	@echo "✅ Backup created!"

# Restore database
restore:
	@read -p "Enter backup file path: " backup_file; \
	docker-compose exec -T postgres psql -U postgres goquant < $$backup_file
	@echo "✅ Database restored!"
