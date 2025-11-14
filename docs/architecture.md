# GoQuant Oracle System Architecture

## Overview

The GoQuant Oracle System is a manipulation-resistant, high-performance price feed aggregator designed for perpetual futures trading on Solana. It combines multiple oracle sources (Pyth Network and Switchboard) to provide reliable, low-latency price data with built-in validation and consensus mechanisms.

## System Architecture

```
┌─────────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
│   Pyth Network      │    │  Switchboard     │    │   Custom Oracles    │
│   Price Feeds       │    │  Aggregators     │    │   (Future)          │
└─────────┬───────────┘    └─────────┬────────┘    └─────────┬───────────┘
          │                          │                       │
          └──────────────┬───────────────────┬─────────────────┘
                         │                   │
                    ┌────▼─────────────────▼────┐
                    │   Oracle Integration      │
                    │   Smart Contract          │
                    │   (Solana/Anchor)         │
                    └────┬─────────────────┬────┘
                         │                 │
              ┌──────────▼─────────────────▼──────────┐
              │        Price Aggregator               │
              │     (Rust Backend Service)            │
              │                                       │
              │  • Consensus Validation               │
              │  • Deviation Detection                │
              │  • Weighted Averaging                 │
              │  • Staleness Checks                   │
              └──────┬─────────────────┬──────────────┘
                     │                 │
         ┌───────────▼─────┐    ┌──────▼──────┐
         │   PostgreSQL    │    │   Redis     │
         │   (Price        │    │   (Price    │
         │    History)     │    │    Cache)   │
         └─────────────────┘    └─────────────┘
                     │                 │
              ┌──────▼─────────────────▼──────────┐
              │         REST API                  │
              │      WebSocket Server             │
              │                                   │
              │  • Real-time price feeds          │
              │  • Historical data                │
              │  • Health monitoring              │
              │  • Funding rate calculations      │
              └───────────────────────────────────┘
```

## Key Components

### 1. Oracle Integration Smart Contract (`programs/oracle-integration/`)

**Purpose**: On-chain price validation and aggregation
- **Language**: Rust + Anchor Framework
- **Functions**:
  - `initialize_oracle()`: Setup oracle configuration
  - `fetch_aggregated_price()`: Validate and aggregate prices from multiple sources
  - Consensus validation with configurable deviation thresholds
  - Price staleness detection

### 2. Price Aggregator Backend (`backend/`)

**Purpose**: Off-chain price processing and API service
- **Language**: Rust + Tokio async runtime
- **Components**:
  - **Oracle Client**: Interfaces with Solana RPC to fetch on-chain price data
  - **Price Aggregator**: Implements weighted averaging and validation logic
  - **WebSocket Server**: Real-time price streaming to clients
  - **REST API**: HTTP endpoints for price queries and system health

### 3. Database Layer (`db/`)

**Purpose**: Persistent storage for price history and system metrics
- **Primary Store**: PostgreSQL for price history, oracle metadata, and trading pair configurations
- **Cache Layer**: Redis for low-latency price access and WebSocket state management

## Data Flow

1. **Price Collection**:
   - Pyth Network feeds are read directly from Solana accounts
   - Switchboard aggregators are queried via their on-chain programs
   - Raw price data is validated for freshness and confidence levels

2. **Consensus & Validation**:
   - Prices from different sources are compared for deviation
   - Configurable thresholds prevent manipulation (default: 5% max deviation)
   - Weighted averaging combines prices (Pyth: 60%, Switchboard: 40%)

3. **Distribution**:
   - Mark prices are cached in Redis for <1ms access
   - WebSocket clients receive real-time updates
   - Historical data is stored in PostgreSQL for analytics

## Security Features

### Manipulation Resistance
- **Multi-source aggregation**: No single oracle can manipulate prices
- **Deviation monitoring**: Large price differences trigger alerts
- **Time-weighted validation**: Prevents flash-loan style attacks
- **Confidence scoring**: Lower confidence scores for suspicious data

### System Reliability
- **Graceful degradation**: System continues with single oracle if one fails
- **Circuit breakers**: Automatic failover when oracle sources are stale
- **Health monitoring**: Comprehensive system health checks and metrics

## Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| Price Update Latency | < 500ms | TBD |
| API Response Time | < 50ms | TBD |
| WebSocket Latency | < 100ms | TBD |
| System Uptime | 99.9% | TBD |
| Throughput | 10k+ req/sec | TBD |

## API Endpoints

### REST API

```
GET  /health                    # System health check
GET  /api/v1/price?symbol=BTC   # Get latest price for symbol
GET  /api/v1/prices             # Get all supported prices
GET  /api/v1/history            # Get price history
GET  /api/v1/funding            # Get funding rates
```

### WebSocket API

```
ws://localhost:3001/ws          # Real-time price feed
```

**Message Format**:
```json
{
  "type": "price_update",
  "symbol": "BTC/USD",
  "mark_price": 45000.50,
  "index_price": 44995.25,
  "funding_rate": 0.0001,
  "confidence": 0.98,
  "timestamp": 1699123456789
}
```

## Configuration

The system uses TOML configuration files for different environments:

- `config/dev.toml`: Development environment
- `config/prod.toml`: Production environment (to be created)
- `config/test.toml`: Testing environment (to be created)

## Monitoring & Alerting

### Health Checks
- Database connectivity
- Redis availability
- Oracle feed freshness
- Price deviation alerts
- System performance metrics

### Metrics Collection
- Price update frequency
- API request latency
- WebSocket connection count
- Database query performance
- Cache hit ratios

## Deployment

The system is containerized using Docker and can be deployed with:

```bash
docker-compose up -d
```

This starts:
- PostgreSQL database
- Redis cache
- Oracle backend service
- Monitoring stack (Prometheus/Grafana)

## Development Setup

1. **Prerequisites**:
   - Rust 1.75+
   - Solana CLI
   - PostgreSQL 15+
   - Redis 6+
   - Docker & Docker Compose

2. **Installation**:
   ```bash
   # Clone repository
   git clone <repo-url>
   cd goquant-oracle-system
   
   # Start infrastructure
   docker-compose up -d postgres redis
   
   # Setup database
   psql -f db/schema.sql
   
   # Build and run
   cargo build --release
   cargo run --bin goquant-oracle-backend
   ```

## Future Enhancements

1. **Additional Oracle Sources**:
   - Chainlink price feeds
   - Custom market maker feeds
   - Cross-chain oracle bridges

2. **Advanced Features**:
   - Machine learning anomaly detection
   - Dynamic weight adjustment
   - Cross-market arbitrage detection
   - MEV protection mechanisms

3. **Scalability**:
   - Horizontal scaling with load balancers
   - Multi-region deployment
   - Read replicas for high availability

## License

This project is licensed under the MIT License. See LICENSE file for details.
