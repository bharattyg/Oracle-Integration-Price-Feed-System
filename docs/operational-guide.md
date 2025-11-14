# GoQuant Oracle System - Operational Guide

## Overview

This operational guide provides step-by-step instructions for managing the GoQuant Oracle System in production. It covers common administrative tasks, monitoring procedures, troubleshooting, and best practices for maintaining optimal system performance.

## Adding New Symbols

### Prerequisites
- Database access with write permissions
- Backend service admin credentials
- Oracle feed configurations for new symbols

### Step 1: Database Configuration

Add the new symbol to the database configuration:

```sql
-- Add symbol to supported_symbols table
INSERT INTO supported_symbols (
    symbol,
    base_asset,
    quote_asset,
    enabled,
    min_sources,
    max_deviation_bps,
    staleness_threshold_ms
) VALUES (
    'AVAX/USD',
    'AVAX',
    'USD',
    true,
    2,
    500,  -- 5% max deviation
    30000 -- 30 second staleness threshold
);

-- Add oracle source mappings
INSERT INTO oracle_symbol_mappings (
    symbol,
    oracle_source,
    external_id,
    enabled,
    weight
) VALUES 
(
    'AVAX/USD',
    'Pyth',
    '0x93da3352f9f1d105fdfe4971cfa80e9dd777bfc5d0f683ebb6e1294b92137bb7',
    true,
    0.6
),
(
    'AVAX/USD', 
    'Switchboard',
    'FGXWpJ7NzHEJq9fvC2pBpjgxJcMfE5oUgNhME4K3vBJ5',
    true,
    0.4
);
```

### Step 2: Configuration Update

Update the runtime configuration file:

```json
{
  "symbol_configs": {
    "AVAX/USD": {
      "enabled": true,
      "min_sources": 2,
      "max_deviation_percent": 5.0,
      "staleness_threshold_ms": 30000,
      "confidence_threshold": 2.0
    }
  }
}
```

### Step 3: Service Restart and Validation

```bash
# Reload configuration (if dynamic reload is supported)
curl -X POST http://localhost:3000/admin/reload-config \
  -H "Authorization: Bearer $ADMIN_TOKEN"

# Or restart the service
sudo systemctl restart goquant-oracle

# Verify the symbol is available
curl http://localhost:3000/api/v1/price/AVAX-USD

# Check system health includes new symbol
curl http://localhost:3000/api/v1/system/health | jq '.oracle_sources'
```

### Step 4: Monitoring Setup

Add the new symbol to monitoring dashboards:

```yaml
# Prometheus alert rules
groups:
- name: avax_alerts
  rules:
  - alert: AVAXPriceStale
    expr: (time() - oracle_price_timestamp{symbol="AVAX/USD"}) > 60
    for: 2m
    labels:
      severity: warning
    annotations:
      summary: "AVAX/USD price data is stale"
      
  - alert: AVAXHighDeviation
    expr: oracle_price_deviation_percent{symbol="AVAX/USD"} > 5
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "AVAX/USD price deviation exceeds threshold"
```

### Verification Checklist

- [ ] Symbol appears in `/api/v1/prices` endpoint
- [ ] Price updates are received from all configured oracle sources  
- [ ] WebSocket subscriptions work for the new symbol
- [ ] Manipulation detection is active
- [ ] Monitoring alerts are configured
- [ ] Database entries are correct

## Oracle Health Monitoring

### Real-time Health Dashboard

Monitor oracle health through multiple channels:

```bash
# Quick health check
curl -s http://localhost:3000/api/v1/system/health | jq '
{
  status: .status,
  oracle_sources: [.oracle_sources[] | {
    name: .name,
    status: .status, 
    response_time: .response_time_ms,
    success_rate: .success_rate_24h
  }]
}'

# Detailed oracle metrics
curl -s http://localhost:3000/metrics | grep oracle_
```

### Key Health Metrics

#### Oracle Response Times
```bash
# Monitor average response times per oracle
curl -s http://localhost:3000/metrics | grep 'oracle_response_time_avg'

# Alert if response time > 1000ms consistently
oracle_response_time_avg{source="Pyth"} > 1000
oracle_response_time_avg{source="Switchboard"} > 1000
```

#### Success Rates
```bash
# Monitor oracle success rates
curl -s http://localhost:3000/api/v1/system/health | jq '
.oracle_sources[] | select(.success_rate_24h < 0.95) | {
  name: .name,
  success_rate: .success_rate_24h,
  status: .status
}'
```

#### Data Quality Metrics
```bash
# Check manipulation scores
curl -s http://localhost:3000/api/v1/manipulation/BTC-USD | jq '
{
  symbol: .symbol,
  current_score: .current_score,
  risk_level: .risk_level,
  recent_events: .detected_events | length
}'
```

### Automated Health Monitoring Script

```bash
#!/bin/bash
# oracle_health_monitor.sh

ORACLE_ENDPOINT="http://localhost:3000"
ALERT_WEBHOOK="$SLACK_WEBHOOK_URL"
LOG_FILE="/var/log/goquant/health_monitor.log"

check_oracle_health() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    local health_data=$(curl -s "$ORACLE_ENDPOINT/api/v1/system/health")
    
    if [ $? -ne 0 ]; then
        echo "[$timestamp] ERROR: Unable to connect to oracle service" >> "$LOG_FILE"
        send_alert "🚨 Oracle service unreachable"
        return 1
    fi
    
    local overall_status=$(echo "$health_data" | jq -r '.status')
    
    if [ "$overall_status" != "Healthy" ]; then
        echo "[$timestamp] WARNING: System status is $overall_status" >> "$LOG_FILE"
        
        # Get detailed component status
        local failing_components=$(echo "$health_data" | jq -r '
            .oracle_sources[] | 
            select(.status != "Healthy") | 
            "\(.name): \(.status)"
        ')
        
        send_alert "⚠️ Oracle health degraded: $failing_components"
    fi
    
    # Check individual oracle response times
    echo "$health_data" | jq -r '.oracle_sources[] | 
        select(.response_time_ms > 1000) | 
        "\(.name) response time: \(.response_time_ms)ms"' | \
    while read line; do
        if [ -n "$line" ]; then
            echo "[$timestamp] WARNING: $line" >> "$LOG_FILE"
            send_alert "🐌 Slow oracle response: $line"
        fi
    done
}

send_alert() {
    local message="$1"
    if [ -n "$ALERT_WEBHOOK" ]; then
        curl -X POST "$ALERT_WEBHOOK" \
            -H 'Content-type: application/json' \
            --data "{\"text\":\"$message\"}"
    fi
}

# Run health check
check_oracle_health

# Add to crontab for regular monitoring
# */2 * * * * /usr/local/bin/oracle_health_monitor.sh
```

## Handling Oracle Outages

### Identifying Oracle Outages

#### Automated Detection
The system automatically detects oracle outages through multiple indicators:

1. **Response timeouts** - No response within 5 seconds
2. **HTTP errors** - 4xx/5xx status codes from oracle APIs
3. **Data quality issues** - Invalid or malformed responses
4. **Circuit breaker activation** - Automatic failover when error rate > 50%

#### Manual Verification
```bash
# Check specific oracle status
curl -s http://localhost:3000/api/v1/system/health | jq '
.oracle_sources[] | select(.name == "Pyth") | {
  status: .status,
  last_update: .last_update,
  error_count: .error_count_1h
}'

# Test oracle endpoints directly
curl -s "https://hermes.pyth.network/api/latest_price_feeds?ids[]=0xe62df6c8b4c85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43"

# Check Solana RPC for Switchboard
curl -s https://api.mainnet-beta.solana.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

### Failover Procedures

#### Automatic Failover
The system implements automatic failover mechanisms:

```rust
// Circuit breaker configuration
CircuitBreakerConfig {
    failure_threshold: 0.5,     // 50% failure rate
    recovery_timeout: 300,      // 5 minutes
    min_requests: 10,          // Minimum requests before activation
}
```

#### Manual Failover Operations

**Emergency Oracle Disable:**
```bash
# Temporarily disable problematic oracle source
curl -X POST http://localhost:3000/admin/oracle/disable \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "source": "Pyth",
    "reason": "API outage detected",
    "duration_minutes": 30
  }'
```

**Cache-Only Mode:**
```bash
# Switch to cache-only mode during total outage
curl -X POST http://localhost:3000/admin/emergency/cache-mode \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "enabled": true,
    "max_staleness_minutes": 10
  }'
```

**Oracle Weight Adjustment:**
```bash
# Temporarily adjust oracle weights during partial outage
curl -X POST http://localhost:3000/admin/oracle/weights \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "weights": {
      "Pyth": 0.0,
      "Switchboard": 1.0
    }
  }'
```

### Outage Recovery Procedures

#### Gradual Recovery
```bash
# Re-enable oracle source gradually
curl -X POST http://localhost:3000/admin/oracle/enable \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "source": "Pyth",
    "gradual": true,
    "initial_weight": 0.1,
    "ramp_duration_minutes": 15
  }'

# Monitor recovery metrics
watch "curl -s http://localhost:3000/api/v1/system/health | jq '.oracle_sources[] | select(.name==\"Pyth\")'"
```

#### Recovery Validation
```bash
# Validate oracle recovery
./scripts/validate_oracle_recovery.sh Pyth

# Sample validation script
#!/bin/bash
ORACLE_NAME="$1"
ENDPOINT="http://localhost:3000"

echo "Validating $ORACLE_NAME recovery..."

# Check oracle status
STATUS=$(curl -s "$ENDPOINT/api/v1/system/health" | jq -r ".oracle_sources[] | select(.name==\"$ORACLE_NAME\") | .status")

if [ "$STATUS" = "Healthy" ]; then
    echo "✅ $ORACLE_NAME status: $STATUS"
else
    echo "❌ $ORACLE_NAME status: $STATUS"
    exit 1
fi

# Check recent price updates
RECENT_UPDATES=$(curl -s "$ENDPOINT/metrics" | grep "price_updates_total{.*source=\"$ORACLE_NAME\".*}" | tail -1)
echo "📊 Recent updates: $RECENT_UPDATES"

# Verify price consistency
for SYMBOL in BTC/USD ETH/USD SOL/USD; do
    PRICE_DATA=$(curl -s "$ENDPOINT/api/v1/price/${SYMBOL/\//-}")
    SOURCES=$(echo "$PRICE_DATA" | jq -r '.sources | join(", ")')
    
    if [[ "$SOURCES" == *"$ORACLE_NAME"* ]]; then
        echo "✅ $SYMBOL includes $ORACLE_NAME"
    else
        echo "⚠️ $SYMBOL missing $ORACLE_NAME"
    fi
done
```

## Debugging Price Issues

### Common Price Issues

#### 1. Stale Prices

**Diagnosis:**
```bash
# Check price timestamps
curl -s http://localhost:3000/api/v1/price/BTC-USD | jq '
{
  symbol: .symbol,
  price: .price,
  age_seconds: (now - .timestamp),
  sources: .sources
}'

# Check staleness across all symbols
curl -s http://localhost:3000/api/v1/prices | jq '
map(select((now - .timestamp) > 60)) | 
map({symbol: .symbol, age_seconds: (now - .timestamp)})'
```

**Resolution:**
```bash
# Force price refresh for specific symbol
curl -X POST http://localhost:3000/admin/prices/refresh \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"symbols": ["BTC/USD"]}'

# Clear cache for stale symbol
curl -X DELETE http://localhost:3000/admin/cache/price/BTC-USD \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

#### 2. Price Deviation Issues

**Diagnosis:**
```bash
# Check price sources and deviations
curl -s http://localhost:3000/api/v1/price/BTC-USD/sources | jq '
{
  symbol: .symbol,
  consensus_price: .consensus_price,
  sources: [.sources[] | {
    name: .name,
    price: .price,
    deviation_percent: .deviation_percent,
    weight: .weight
  }]
}'

# Check manipulation detection
curl -s http://localhost:3000/api/v1/manipulation/BTC-USD | jq '
{
  current_score: .current_score,
  risk_level: .risk_level,
  recent_events: [.detected_events[] | {
    timestamp: .timestamp,
    type: .event_type,
    score: .manipulation_score
  }]
}'
```

**Resolution:**
```bash
# Temporarily increase deviation threshold
curl -X POST http://localhost:3000/admin/config/update \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "symbol_configs": {
      "BTC/USD": {
        "max_deviation_percent": 10.0
      }
    }
  }'

# Manually trigger outlier filtering
curl -X POST http://localhost:3000/admin/prices/filter-outliers \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"symbol": "BTC/USD"}'
```

#### 3. Missing Oracle Sources

**Diagnosis:**
```bash
# Check which sources are contributing to price
curl -s http://localhost:3000/api/v1/price/ETH-USD | jq '.sources'

# Check oracle source status
curl -s http://localhost:3000/api/v1/system/health | jq '
.oracle_sources[] | {
  name: .name,
  status: .status,
  last_successful_update: .last_update,
  error_rate: .error_count_1h
}'
```

**Resolution:**
```bash
# Reset oracle connection
curl -X POST http://localhost:3000/admin/oracle/reset \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"source": "Switchboard"}'

# Force oracle re-authentication
curl -X POST http://localhost:3000/admin/oracle/reauth \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"source": "Pyth"}'
```

### Debugging Tools

#### Log Analysis
```bash
# Monitor real-time logs
tail -f /var/log/goquant/oracle.log | jq 'select(.level == "ERROR")'

# Search for specific symbol issues
grep "BTC/USD" /var/log/goquant/oracle.log | grep "ERROR" | tail -20

# Analyze oracle response patterns
grep "oracle_response" /var/log/goquant/oracle.log | \
jq 'select(.oracle_source == "Pyth") | {timestamp: .timestamp, duration_ms: .duration_ms, status: .status}'
```

#### Performance Analysis
```bash
# Check price aggregation performance
curl -s http://localhost:3000/metrics | grep "aggregation_duration"

# Monitor database query performance  
curl -s http://localhost:3000/admin/db/slow-queries \
  -H "Authorization: Bearer $ADMIN_TOKEN"

# WebSocket connection diagnostics
curl -s http://localhost:3000/admin/websocket/stats \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq '
{
  active_connections: .active_connections,
  total_subscriptions: .total_subscriptions,
  average_latency_ms: .average_latency_ms
}'
```

#### Emergency Procedures

**Emergency Price Override:**
```bash
# Manually set price during oracle failures
curl -X POST http://localhost:3000/admin/emergency/price-override \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "symbol": "BTC/USD",
    "price": 65000.0,
    "source": "manual",
    "reason": "oracle outage emergency override",
    "duration_minutes": 30
  }'
```

**Emergency System Halt:**
```bash
# Halt all price updates during major issues
curl -X POST http://localhost:3000/admin/emergency/halt \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "reason": "investigating manipulation attack",
    "notify_clients": true
  }'

# Resume normal operations
curl -X POST http://localhost:3000/admin/emergency/resume \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

### Troubleshooting Playbook

#### Issue: All Prices Stale
1. Check system clock synchronization: `timedatectl status`
2. Verify internet connectivity: `ping 8.8.8.8`
3. Test oracle endpoints directly
4. Check database connectivity
5. Restart service if necessary

#### Issue: High Manipulation Scores
1. Verify market conditions with external sources
2. Check for oracle feed issues
3. Review manipulation detection thresholds
4. Analyze price velocity and deviation patterns
5. Consider temporary threshold adjustment

#### Issue: Poor Performance
1. Monitor system resources: `htop`, `iotop`
2. Check database performance: slow query log
3. Analyze oracle response times
4. Review connection pool utilization
5. Consider scaling up infrastructure

## Best Practices

### Regular Maintenance

**Daily Tasks:**
- Review health dashboard
- Check manipulation detection alerts  
- Verify all symbols are updating
- Monitor system performance metrics

**Weekly Tasks:**
- Analyze oracle source reliability metrics
- Review configuration for optimization opportunities
- Update oracle feed mappings as needed
- Performance tune database queries

**Monthly Tasks:**
- Review and update manipulation detection thresholds
- Analyze historical data quality trends
- Update documentation and runbooks
- Plan capacity upgrades based on usage growth

### Security Considerations

- Rotate API keys regularly
- Monitor for unusual price patterns
- Implement IP allowlists for admin endpoints
- Regular security audits of oracle connections
- Backup and test disaster recovery procedures

This operational guide provides the foundation for reliable, secure operation of the GoQuant Oracle System in production environments.
