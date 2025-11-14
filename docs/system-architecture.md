# GoQuant Oracle System Architecture

## Overview

The GoQuant Oracle System is a high-performance, multi-source price aggregation platform designed for perpetual futures trading. It combines real-time data from multiple oracle providers (Pyth Network and Switchboard) with advanced manipulation detection and failover mechanisms to ensure reliable, accurate pricing for trading operations.

## System Components

### 1. Oracle Integration Layer

The oracle integration layer serves as the foundation for multi-source price aggregation:

```
┌─────────────────────────────────────────────────────────────┐
│                   Oracle Integration Layer                  │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │    Pyth     │    │ Switchboard │    │   Internal  │     │
│  │   Client    │    │   Client    │    │   Pricing   │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│         │                   │                   │          │
│         └───────────────────┼───────────────────┘          │
│                            │                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │            Price Aggregation Engine                     │ │
│  │  • Consensus calculation                                │ │
│  │  • Outlier detection                                    │ │
│  │  • Confidence weighting                                 │ │
│  │  • Time-weighted averaging                              │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 2. Price Aggregation Flow

The system follows a sophisticated multi-stage price aggregation process:

```mermaid
graph TD
    A[Oracle Sources] --> B[Data Collection]
    B --> C[Price Normalization]
    C --> D[Confidence Analysis]
    D --> E[Outlier Detection]
    E --> F[Consensus Calculation]
    F --> G[Manipulation Detection]
    G --> H[Final Price Output]
    
    I[Cache Layer] --> J[Database Storage]
    H --> I
    H --> K[WebSocket Broadcasting]
    H --> L[REST API Endpoints]
```

#### Stage-by-Stage Process:

1. **Data Collection**: Parallel fetching from all configured oracle sources
2. **Price Normalization**: Converting raw feeds to standardized decimal format
3. **Confidence Analysis**: Evaluating data quality and reliability metrics
4. **Outlier Detection**: Identifying and filtering anomalous price points
5. **Consensus Calculation**: Computing weighted average based on confidence scores
6. **Manipulation Detection**: Real-time analysis for market manipulation patterns
7. **Output Distribution**: Broadcasting to WebSocket clients and REST endpoints

### 3. Failover Mechanisms

The system implements multiple layers of failover protection:

#### Oracle Source Failover
```
Primary Sources     Failover Logic           Backup Strategies
┌─────────────┐    ┌─────────────────┐     ┌─────────────────┐
│    Pyth     │───▶│ Health Check    │────▶│ Cache Fallback  │
│ Network     │    │ • Response Time │     │ • Last Known    │
│             │    │ • Data Quality  │     │   Good Price    │
└─────────────┘    │ • Confidence    │     │ • Stale Timeout │
                   └─────────────────┘     └─────────────────┘
┌─────────────┐    ┌─────────────────┐     ┌─────────────────┐
│ Switchboard │───▶│ Circuit Breaker │────▶│ Manual Override │
│     V2      │    │ • Error Rate    │     │ • Emergency     │
│             │    │ • Latency       │     │   Pricing       │
└─────────────┘    │ • Availability  │     └─────────────────┘
                   └─────────────────┘
```

#### Circuit Breaker Implementation
- **Threshold**: 50% error rate over 60-second window
- **Recovery**: Gradual traffic restoration after 5 minutes
- **Fallback**: Cached prices with staleness warnings

### 4. Data Flow and Caching Strategy

#### Primary Data Flow
```
[Oracle APIs] ──→ [Rate Limiting] ──→ [Validation] ──→ [Aggregation]
     │                                                      │
     ▼                                                      ▼
[Error Handling] ←─────────────────────────── [Price Database]
     │                                                      │
     ▼                                                      ▼
[Circuit Breaker] ──→ [Cache Fallback] ──→ [Client Distribution]
```

#### Cache Architecture
- **L1 Cache**: Redis (sub-millisecond access, 5-minute TTL)
- **L2 Cache**: PostgreSQL (persistent storage, price history)
- **L3 Cache**: In-memory cache (emergency fallback, 30-second TTL)

#### Performance Specifications
- **Target Latency**: < 500ms end-to-end
- **Cache Hit Ratio**: > 95% for L1 cache
- **Data Freshness**: < 30 seconds for critical pairs
- **Availability**: 99.99% uptime target

### 5. Security and Validation

#### Multi-Layer Validation
```
┌─────────────────────────────────────────────────────────────┐
│                      Validation Pipeline                    │
├─────────────────────────────────────────────────────────────┤
│ 1. Source Authentication                                    │
│    • API key validation                                     │
│    • SSL/TLS verification                                   │
│    • Rate limit enforcement                                 │
├─────────────────────────────────────────────────────────────┤
│ 2. Data Integrity Checks                                    │
│    • Schema validation                                      │
│    • Range validation (min/max bounds)                     │
│    • Timestamp verification                                │
├─────────────────────────────────────────────────────────────┤
│ 3. Manipulation Detection                                   │
│    • Velocity analysis                                      │
│    • Statistical outlier detection                         │
│    • Pattern recognition (pump/dump)                       │
├─────────────────────────────────────────────────────────────┤
│ 4. Consensus Validation                                     │
│    • Cross-source comparison                               │
│    • Confidence score weighting                            │
│    • Deviation threshold enforcement                       │
└─────────────────────────────────────────────────────────────┘
```

## Scalability Design

### Horizontal Scaling
- **Microservice Architecture**: Independent oracle client services
- **Load Balancing**: Round-robin distribution across oracle instances
- **Database Sharding**: Symbol-based partitioning for large datasets

### Vertical Scaling
- **Connection Pooling**: Optimized database connection management
- **Async Processing**: Non-blocking I/O for all oracle requests
- **Memory Management**: Efficient price history storage with LRU eviction

## Monitoring and Observability

### Key Metrics
- **Oracle Health**: Response time, error rate, availability per source
- **Price Quality**: Confidence scores, deviation metrics, staleness
- **System Performance**: Latency percentiles, throughput, cache hit rates
- **Business Metrics**: Symbol coverage, manipulation detection rates

### Alerting Thresholds
- **Critical**: Oracle source down > 2 minutes
- **Warning**: Price deviation > 5% between sources
- **Info**: Cache miss rate > 10%

## Deployment Architecture

### Production Environment
```
┌─────────────────────────────────────────────────────────────┐
│                    Load Balancer (NGINX)                    │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Backend   │  │   Backend   │  │   Backend   │         │
│  │ Instance 1  │  │ Instance 2  │  │ Instance 3  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐           ┌─────────────────────────────┐  │
│  │    Redis    │           │        PostgreSQL           │  │
│  │   Cluster   │           │      (Primary/Replica)      │  │
│  └─────────────┘           └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Development Environment
- **Docker Compose**: Single-node development stack
- **Hot Reload**: Automatic service restart on code changes
- **Test Database**: Isolated test environment with mock data

## Configuration Management

### Environment Variables
```env
# Oracle Configuration
PYTH_API_URL=https://hermes.pyth.network
SWITCHBOARD_RPC_URL=https://api.mainnet-beta.solana.com
ORACLE_TIMEOUT_MS=5000

# Database Configuration
DATABASE_URL=postgresql://user:pass@localhost:5432/goquant
REDIS_URL=redis://localhost:6379

# Performance Tuning
MAX_PRICE_HISTORY=1000
CACHE_TTL_SECONDS=300
AGGREGATION_WINDOW_MS=100

# Security Settings
API_RATE_LIMIT=1000
MANIPULATION_THRESHOLD=0.7
CIRCUIT_BREAKER_THRESHOLD=0.5
```

### Symbol Configuration
Symbols are configured via database tables with support for:
- **Dynamic Addition**: Add new trading pairs without service restart
- **Oracle Mapping**: Configure which oracles provide data for each symbol
- **Validation Rules**: Set price bounds and validation parameters per symbol

This architecture ensures high availability, accurate pricing, and robust operation even under adverse market conditions while maintaining the flexibility to scale and adapt to changing requirements.
