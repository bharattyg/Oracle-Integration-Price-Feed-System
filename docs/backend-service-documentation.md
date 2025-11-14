# Backend Service Documentation

## Overview

The GoQuant Oracle Backend is a high-performance Rust service built with Axum that provides REST APIs and WebSocket endpoints for real-time oracle price data. It serves as the central hub for price aggregation, manipulation detection, and client distribution.

## Module Architecture

### Core Modules

```
src/
├── main.rs                    # Application entry point and HTTP server
├── oracle_client.rs           # Oracle provider integrations  
├── price_aggregator.rs        # Price aggregation and consensus logic
└── tests/                     # Comprehensive test suites
    ├── price_parsing_tests.rs
    ├── integration_tests.rs
    ├── mock_oracle_tests.rs
    ├── chaos_tests.rs
    └── manipulation_detection_tests.rs
```

### Module Responsibilities

#### main.rs - Application Server
- **HTTP Server Setup**: Axum-based REST API server with middleware
- **Route Configuration**: API endpoint definitions and handlers
- **Database Integration**: PostgreSQL connection pooling and queries
- **Configuration Management**: Environment-based configuration loading
- **Middleware Stack**: CORS, logging, rate limiting, and error handling

```rust
// Server configuration structure
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub server_port: u16,
    pub log_level: String,
    pub solana_rpc_url: String,
    pub pyth_api_url: String,
}

// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub redis_client: RedisClient,
    pub price_aggregator: Arc<PriceAggregator>,
    pub oracle_manager: Arc<OracleManager>,
    pub config: AppConfig,
}
```

#### oracle_client.rs - Oracle Integrations
- **Oracle Trait Definition**: Common interface for all oracle providers
- **Pyth Client**: HTTP API integration with Hermes endpoints
- **Switchboard Client**: Solana RPC integration for on-chain data
- **Connection Management**: Connection pooling and retry logic
- **Error Handling**: Provider-specific error mapping and recovery

```rust
// Unified oracle client trait
#[async_trait]
pub trait OracleClient: Send + Sync {
    async fn get_price(&self, symbol: &str) -> Result<PriceData>;
    async fn get_multiple_prices(&self, symbols: &[String]) -> Result<Vec<PriceData>>;
    fn get_name(&self) -> &str;
}

// Price data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub symbol: String,
    pub price: f64,
    pub confidence: f64,
    pub timestamp: i64,
    pub source: String,
}
```

#### price_aggregator.rs - Aggregation Engine
- **Consensus Calculation**: Multi-source price aggregation with weighting
- **Manipulation Detection**: Real-time analysis for market manipulation
- **Quality Assessment**: Data quality scoring and validation
- **Circuit Breaker**: Automatic failover mechanisms for oracle outages
- **Cache Management**: Redis integration for performance optimization

```rust
// Price aggregator with built-in manipulation detection
pub struct PriceAggregator {
    pub oracle_clients: Vec<Box<dyn OracleClient>>,
    pub manipulation_detector: ManipulationDetector,
    pub circuit_breaker: CircuitBreaker,
    pub cache: RedisClient,
}

// Aggregated price result with metadata
#[derive(Debug, Clone, Serialize)]
pub struct AggregatedPrice {
    pub symbol: String,
    pub mark_price: f64,
    pub index_price: f64,
    pub confidence_interval: f64,
    pub last_updated: i64,
    pub source_count: u32,
    pub quality_score: f64,
    pub manipulation_score: f64,
}
```

## API Specifications

### REST API Endpoints

#### Core Price Endpoints

**GET /api/v1/price/{symbol}**
```rust
// Get current aggregated price for a symbol
async fn get_price(
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<PriceResponse>, StatusCode>

// Response format
#[derive(Serialize)]
pub struct PriceResponse {
    pub symbol: String,
    pub price: f64,
    pub confidence: f64,
    pub timestamp: i64,
    pub sources: Vec<String>,
    pub quality_score: f64,
}
```

**GET /api/v1/prices**
```rust
// Get prices for multiple symbols
async fn get_multiple_prices(
    Query(params): Query<MultiPriceRequest>,
    State(state): State<AppState>,
) -> Result<Json<Vec<PriceResponse>>, StatusCode>

#[derive(Deserialize)]
pub struct MultiPriceRequest {
    pub symbols: String, // Comma-separated list
    pub sources: Option<String>, // Optional source filtering
}
```

#### Advanced Trading Endpoints

**GET /api/v1/funding/{symbol}**
```rust
// Calculate funding rate for perpetual futures
async fn get_funding_rate(
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<FundingRateResponse>, StatusCode>

#[derive(Serialize)]
pub struct FundingRateResponse {
    pub symbol: String,
    pub funding_rate: f64,
    pub funding_rate_8h: f64,
    pub next_funding_time: i64,
    pub mark_price: f64,
    pub index_price: f64,
    pub interest_rate: f64,
    pub premium_rate: f64,
}
```

**GET /api/v1/liquidation/{symbol}**
```rust
// Calculate liquidation prices for positions
async fn get_liquidation_price(
    Path(symbol): Path<String>,
    Query(params): Query<LiquidationRequest>,
    State(state): State<AppState>,
) -> Result<Json<LiquidationResponse>, StatusCode>

#[derive(Deserialize)]
pub struct LiquidationRequest {
    pub position_size: f64,
    pub entry_price: f64,
    pub leverage: f64,
    pub side: String, // "long" or "short"
}

#[derive(Serialize)]
pub struct LiquidationResponse {
    pub symbol: String,
    pub liquidation_price: f64,
    pub maintenance_margin: f64,
    pub unrealized_pnl: f64,
    pub margin_ratio: f64,
}
```

#### System Health and Monitoring

**GET /api/v1/system/health**
```rust
// Comprehensive system health check
async fn get_system_health(
    State(state): State<AppState>,
) -> Result<Json<SystemHealthResponse>, StatusCode>

#[derive(Serialize)]
pub struct SystemHealthResponse {
    pub status: String,
    pub timestamp: i64,
    pub uptime_seconds: u64,
    pub oracle_sources: Vec<OracleHealthStatus>,
    pub database_status: DatabaseStatus,
    pub cache_status: CacheStatus,
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Serialize)]
pub struct OracleHealthStatus {
    pub name: String,
    pub status: String,
    pub last_update: i64,
    pub response_time_ms: u32,
    pub success_rate_24h: f64,
    pub error_count_1h: u32,
}
```

**GET /api/v1/manipulation/{symbol}**
```rust
// Get manipulation detection analysis
async fn get_manipulation_report(
    Path(symbol): Path<String>,
    Query(params): Query<ManipulationQuery>,
    State(state): State<AppState>,
) -> Result<Json<ManipulationReport>, StatusCode>

#[derive(Deserialize)]
pub struct ManipulationQuery {
    pub hours: Option<u32>, // Analysis window (default: 24)
}

#[derive(Serialize)]
pub struct ManipulationReport {
    pub symbol: String,
    pub current_score: f64,
    pub risk_level: String,
    pub analysis_window_hours: u32,
    pub detected_events: Vec<ManipulationEvent>,
    pub price_velocity: f64,
    pub volatility_score: f64,
    pub outlier_count: u32,
}
```

### Error Response Format

Standardized error responses across all endpoints:

```rust
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: i64,
    pub request_id: String,
}

// HTTP status code mapping
pub fn map_error_to_status(error: &anyhow::Error) -> StatusCode {
    if error.to_string().contains("not found") {
        StatusCode::NOT_FOUND
    } else if error.to_string().contains("timeout") {
        StatusCode::REQUEST_TIMEOUT
    } else if error.to_string().contains("rate limit") {
        StatusCode::TOO_MANY_REQUESTS
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
```

## WebSocket Protocols

### Real-time Price Streaming

The WebSocket implementation provides low-latency price updates for real-time trading applications:

```rust
// WebSocket connection handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: AppState) {
    // Client subscription management
    let mut subscriptions: HashSet<String> = HashSet::new();
    let mut last_prices: HashMap<String, PriceData> = HashMap::new();
    
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(request) = serde_json::from_str::<WebSocketRequest>(&text) {
                    match request.action.as_str() {
                        "subscribe" => {
                            for symbol in request.symbols {
                                subscriptions.insert(symbol.clone());
                                
                                // Send current price immediately
                                if let Ok(price) = state.price_aggregator
                                    .get_aggregated_price(&symbol).await {
                                    
                                    let response = WebSocketResponse {
                                        type_: "price_update".to_string(),
                                        symbol: symbol.clone(),
                                        data: price,
                                        timestamp: chrono::Utc::now().timestamp(),
                                    };
                                    
                                    if socket.send(Message::Text(
                                        serde_json::to_string(&response).unwrap()
                                    )).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        },
                        "unsubscribe" => {
                            for symbol in request.symbols {
                                subscriptions.remove(&symbol);
                            }
                        },
                        _ => {
                            let error_response = WebSocketError {
                                error: "unknown_action".to_string(),
                                message: format!("Unknown action: {}", request.action),
                            };
                            let _ = socket.send(Message::Text(
                                serde_json::to_string(&error_response).unwrap()
                            )).await;
                        }
                    }
                }
            },
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }
}
```

### WebSocket Message Formats

**Client Request Format:**
```json
{
  "action": "subscribe",
  "symbols": ["BTC/USD", "ETH/USD", "SOL/USD"],
  "options": {
    "throttle_ms": 100,
    "include_metadata": true
  }
}
```

**Server Response Format:**
```json
{
  "type": "price_update",
  "symbol": "BTC/USD",
  "data": {
    "price": 65432.10,
    "confidence": 0.05,
    "timestamp": 1731628800,
    "sources": ["Pyth", "Switchboard"],
    "quality_score": 0.95
  },
  "timestamp": 1731628800
}
```

## Configuration Parameters

### Environment Variables

Complete configuration reference for deployment:

```bash
# Server Configuration
PORT=3000                                    # HTTP server port
HOST=0.0.0.0                                # Bind address
RUST_LOG=info                               # Logging level

# Database Configuration
DATABASE_URL=postgresql://user:pass@localhost:5432/goquant
DATABASE_MAX_CONNECTIONS=20                 # Connection pool size
DATABASE_TIMEOUT_SECONDS=30                # Query timeout

# Redis Configuration  
REDIS_URL=redis://localhost:6379           # Cache connection
REDIS_MAX_CONNECTIONS=10                   # Connection pool size
REDIS_DEFAULT_TTL=300                      # Default cache TTL (seconds)

# Oracle Configuration
PYTH_API_URL=https://hermes.pyth.network   # Pyth Hermes endpoint
SWITCHBOARD_RPC_URL=https://api.mainnet-beta.solana.com # Solana RPC
ORACLE_TIMEOUT_MS=5000                     # Request timeout
ORACLE_RETRY_COUNT=3                       # Retry attempts
ORACLE_RATE_LIMIT=100                      # Requests per minute per source

# Performance Tuning
MAX_PRICE_HISTORY=1000                     # In-memory price history size
AGGREGATION_WINDOW_MS=100                  # Price aggregation interval
WEBSOCKET_MAX_CONNECTIONS=1000             # Concurrent WebSocket limit
CIRCUIT_BREAKER_FAILURE_THRESHOLD=0.5      # 50% failure rate threshold
CIRCUIT_BREAKER_RECOVERY_TIME_MS=300000    # 5 minute recovery period

# Security Configuration
API_RATE_LIMIT_PER_MINUTE=1000             # API rate limiting
CORS_ALLOWED_ORIGINS=*                     # CORS configuration
MANIPULATION_DETECTION_THRESHOLD=0.7        # Manipulation alert threshold

# Monitoring Configuration
METRICS_ENABLED=true                       # Enable Prometheus metrics
HEALTH_CHECK_INTERVAL_MS=30000            # Health check frequency
LOG_STRUCTURED=true                        # Structured JSON logging
```

### Runtime Configuration

Dynamic configuration that can be updated without restart:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    // Oracle weights for consensus calculation
    pub oracle_weights: HashMap<String, f64>,
    
    // Symbol-specific configuration
    pub symbol_configs: HashMap<String, SymbolConfig>,
    
    // Manipulation detection parameters
    pub manipulation_config: ManipulationConfig,
    
    // Performance thresholds
    pub performance_thresholds: PerformanceThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolConfig {
    pub enabled: bool,
    pub min_sources: u32,
    pub max_deviation_percent: f64,
    pub staleness_threshold_ms: u64,
    pub confidence_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManipulationConfig {
    pub velocity_threshold: f64,
    pub volatility_threshold: f64,
    pub pattern_detection_enabled: bool,
    pub outlier_sensitivity: f64,
}
```

## Performance Monitoring

### Metrics Collection

The service exposes Prometheus metrics for comprehensive monitoring:

```rust
// Custom metrics definitions
lazy_static! {
    static ref PRICE_UPDATE_COUNTER: IntCounterVec = IntCounterVec::new(
        Opts::new("price_updates_total", "Total price updates processed"),
        &["symbol", "source", "status"]
    ).unwrap();
    
    static ref RESPONSE_TIME_HISTOGRAM: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request duration")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
        &["method", "endpoint", "status"]
    ).unwrap();
    
    static ref ORACLE_HEALTH_GAUGE: GaugeVec = GaugeVec::new(
        Opts::new("oracle_health_score", "Oracle health scores (0-1)"),
        &["source"]
    ).unwrap();
    
    static ref MANIPULATION_SCORE_GAUGE: GaugeVec = GaugeVec::new(
        Opts::new("manipulation_score", "Current manipulation detection scores"),
        &["symbol"]
    ).unwrap();
}

// Metrics middleware for HTTP requests
pub async fn metrics_middleware(
    req: Request,
    next: Next<AppState>,
) -> impl IntoResponse {
    let start_time = std::time::Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    
    let response = next.run(req).await;
    
    let duration = start_time.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();
    
    RESPONSE_TIME_HISTOGRAM
        .with_label_values(&[method.as_str(), &path, &status])
        .observe(duration);
    
    response
}
```

### Health Check Implementation

Comprehensive health monitoring across all system components:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    pub overall_status: HealthStatus,
    pub components: Vec<ComponentHealth>,
    pub timestamp: i64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub details: serde_json::Value,
    pub last_check: i64,
}

#[derive(Debug, Clone, Serialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl AppState {
    pub async fn perform_health_check(&self) -> HealthCheck {
        let start_time = std::time::Instant::now();
        let mut components = Vec::new();
        
        // Database health
        components.push(self.check_database_health().await);
        
        // Redis health  
        components.push(self.check_redis_health().await);
        
        // Oracle sources health
        for oracle in &self.price_aggregator.oracle_clients {
            components.push(self.check_oracle_health(oracle).await);
        }
        
        // Determine overall status
        let overall_status = if components.iter().any(|c| matches!(c.status, HealthStatus::Unhealthy)) {
            HealthStatus::Unhealthy
        } else if components.iter().any(|c| matches!(c.status, HealthStatus::Degraded)) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        
        HealthCheck {
            overall_status,
            components,
            timestamp: chrono::Utc::now().timestamp(),
            uptime_seconds: start_time.elapsed().as_secs(),
        }
    }
}
```

This backend service provides a robust, scalable foundation for high-frequency oracle price aggregation with comprehensive monitoring, real-time distribution, and advanced manipulation detection capabilities.
