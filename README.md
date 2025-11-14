# GoQuant Oracle & Price Feed System

A manipulation-resistant oracle system on Solana that aggregates Pyth + Switchboard prices for perpetual futures trading with sub-500ms latency.

## 🎯 Features

- **Multi-Oracle Aggregation**: Combines Pyth Network and Switchboard price feeds
- **Manipulation Resistance**: Deviation detection and consensus validation
- **High Performance**: <500ms price update latency, <50ms API responses
- **Real-time Streaming**: WebSocket price feeds for live trading applications
- **Comprehensive API**: REST endpoints for prices, history, and funding rates
- **Built-in Monitoring**: Health checks, metrics, and alerting

## 🏗️ Architecture

```
Pyth Network ──┐
               ├─→ Solana Oracle Contract ─→ Price Aggregator ─→ API + WebSocket
Switchboard ───┘                              ↕                    ↕
                                          PostgreSQL           Redis Cache
```

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+
- Solana CLI
- PostgreSQL 15+
- Redis 6+
- Docker & Docker Compose

### Installation

1. **Clone the repository**:
   ```bash
   git clone https://github.com/your-org/goquant-oracle-system
   cd goquant-oracle-system
   ```

2. **Start infrastructure services**:
   ```bash
   docker-compose up -d postgres redis
   ```

3. **Setup database**:
   ```bash
   # Create database and run migrations
   psql -h localhost -U postgres -c "CREATE DATABASE goquant;"
   psql -h localhost -U postgres -d goquant -f db/schema.sql
   ```

4. **Build and deploy Solana program**:
   ```bash
   cd programs/oracle-integration
   anchor build
   anchor deploy
   ```

5. **Start the backend service**:
   ```bash
   cd backend
   cargo run --release
   ```

## 📊 API Usage

### REST Endpoints

```bash
# Get system health
curl http://localhost:3000/health

# Get latest BTC price
curl http://localhost:3000/api/v1/price?symbol=BTC/USD

# Get all prices
curl http://localhost:3000/api/v1/prices

# Get price history
curl http://localhost:3000/api/v1/history?symbol=BTC/USD&limit=100

# Get funding rate
curl http://localhost:3000/api/v1/funding?symbol=BTC/USD
```

### WebSocket Streaming

```javascript
const ws = new WebSocket('ws://localhost:3001/ws');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Price update:', data);
  // {
  //   "type": "price_update",
  //   "symbol": "BTC/USD",
  //   "mark_price": 45000.50,
  //   "index_price": 44995.25,
  //   "funding_rate": 0.0001,
  //   "confidence": 0.98,
  //   "timestamp": 1699123456789
  // }
};
```

## 🛠️ Development

### Project Structure

```
goquant-oracle-system/
├── programs/oracle-integration/    # Solana smart contract
│   ├── src/lib.rs                 # Oracle aggregation logic
│   ├── tests/                     # Anchor tests
│   ├── Cargo.toml                 # Rust dependencies
│   └── Anchor.toml                # Anchor config
├── backend/                       # Rust backend service
│   ├── src/main.rs               # API server and price aggregator
│   └── Cargo.toml                # Backend dependencies
├── db/schema.sql                  # PostgreSQL schema
├── config/dev.toml               # Development configuration
├── docs/architecture.md          # System documentation
├── docker-compose.yml            # Infrastructure setup
└── README.md                     # This file
```

### Running Tests

```bash
# Test Solana program
cd programs/oracle-integration
anchor test

# Test backend service
cd backend
cargo test
```

### Configuration

Edit `config/dev.toml` to customize:

- Oracle source weights and endpoints
- Price deviation thresholds
- Update intervals
- Database connections
- API rate limits

## 🔒 Security Features

### Manipulation Resistance
- **Multi-source validation**: Aggregates 2+ independent oracles
- **Deviation monitoring**: Alerts on >5% price differences
- **Staleness detection**: Rejects data older than 60 seconds
- **Weighted averaging**: Pyth (60%) + Switchboard (40%)

### System Reliability
- **Graceful degradation**: Continues with single oracle if needed
- **Circuit breakers**: Auto-failover on oracle failures
- **Health monitoring**: Comprehensive system checks
- **Rate limiting**: API protection against abuse

## 📈 Performance Targets

| Metric | Target | Status |
|--------|--------|--------|
| Price Update Latency | < 500ms | ⚡ |
| API Response Time | < 50ms | ⚡ |
| WebSocket Latency | < 100ms | ⚡ |
| System Uptime | 99.9% | 🎯 |
| Throughput | 10k+ req/sec | 📈 |

## 🔧 Configuration

### Supported Trading Pairs

- BTC/USD
- ETH/USD  
- SOL/USD

### Oracle Sources

- **Pyth Network**: 60% weight, sub-second updates
- **Switchboard**: 40% weight, decentralized aggregation

## 📦 Deployment

### Production Deployment

```bash
# Build optimized release
cargo build --release

# Deploy with Docker
docker-compose -f docker-compose.prod.yml up -d

# Monitor logs
docker-compose logs -f oracle-backend
```

### Environment Variables

```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/goquant
REDIS_URL=redis://127.0.0.1:6379
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
LOG_LEVEL=info
```

## 🔍 Monitoring

### Health Endpoints

- `/health` - Overall system health
- `/metrics` - Prometheus metrics (port 9090)

### Key Metrics

- Price update frequency
- Oracle deviation rates  
- API request latency
- WebSocket connection count
- Database performance



## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.


