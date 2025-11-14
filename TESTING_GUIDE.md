# GoQuant Oracle System - Testing Guide

## Prerequisites Check

Before testing, ensure you have:
- [x] Docker Desktop installed and running
- [x] Rust 1.75+ installed
- [x] PostgreSQL client (psql) for direct DB access
- [x] curl or Postman for API testing

## 🐳 Step 1: Start Infrastructure Services

### Start Database & Cache Services
```bash
# Navigate to project root
cd /Users/apple/Desktop/Goquant/goquant-oracle-system

# Start Docker Desktop first!
# Then start PostgreSQL and Redis
docker-compose up -d postgres redis

# Verify containers are running
docker ps

# Check logs
docker-compose logs postgres
docker-compose logs redis
```

### Verify Database Setup
```bash
# Connect to PostgreSQL to verify tables
docker exec -it goquant-postgres psql -U postgres -d goquant

# Once connected, run these SQL commands:
\dt                          # List all tables
\d price_feeds              # Describe price_feeds table structure
\d oracle_sources           # Describe oracle_sources table structure
SELECT * FROM trading_pairs; # View default trading pairs
SELECT * FROM oracle_sources; # View default oracle sources
\q                          # Quit psql
```

### Verify Redis Setup
```bash
# Test Redis connection
docker exec -it goquant-redis redis-cli ping
# Should return "PONG"

# Check Redis info
docker exec -it goquant-redis redis-cli info server
```

## 🦀 Step 2: Build and Run Rust Backend

### Option A: Run Locally (Recommended for Testing)
```bash
cd backend

# Set environment variables
export DATABASE_URL="postgresql://postgres:password@localhost:5432/goquant"
export REDIS_URL="redis://127.0.0.1:6379"
export RUST_LOG=debug
export SOLANA_RPC_URL="https://api.mainnet-beta.solana.com"

# Build and run
cargo build --release
cargo run --release
```

### Option B: Run with Docker
```bash
# From project root
docker-compose up -d oracle-backend

# Check logs
docker-compose logs -f oracle-backend
```

## 🧪 Step 3: API Testing

### Health Check Tests
```bash
# Basic health check
curl http://localhost:3000/health

# Expected response:
# {"status":"healthy","timestamp":1699123456,"database":"connected","oracles":{"BTC/USD":{"price":45000.50,"healthy":true}}}
```

### Price Feed Tests
```bash
# Get single price
curl http://localhost:3000/api/v1/price/BTC/USD
curl http://localhost:3000/api/v1/price/ETH/USD
curl http://localhost:3000/api/v1/price/SOL/USD

# Get multiple prices
curl "http://localhost:3000/api/v1/prices?symbols=BTC/USD,ETH/USD,SOL/USD"

# Get price history (last 24 hours)
curl "http://localhost:3000/api/v1/history?symbol=BTC/USD&hours=24"

# Get manipulation report
curl "http://localhost:3000/api/v1/manipulation?symbol=BTC/USD&hours=1"
```

### WebSocket Testing
```bash
# Test WebSocket connection using wscat (install with: npm install -g wscat)
wscat -c ws://localhost:3001/ws/prices

# You should receive real-time price updates like:
# {"type":"price_update","symbol":"BTC/USD","mark_price":45000.50,"timestamp":1699123456}
```

## 🔍 Step 4: Database Verification

### Check Live Data
```bash
# Connect to database
docker exec -it goquant-postgres psql -U postgres -d goquant

# Verify data is being inserted
SELECT COUNT(*) FROM price_feeds;
SELECT * FROM price_feeds ORDER BY created_at DESC LIMIT 5;

# Check oracle sources
SELECT * FROM oracle_sources WHERE is_active = true;

# View latest prices
SELECT * FROM latest_prices;

# Check oracle health
SELECT * FROM oracle_health;
```

### Monitor Real-time Updates
```sql
-- Watch for new price insertions (run this and leave it open)
SELECT 
    symbol, 
    mark_price, 
    confidence, 
    source_count,
    created_at 
FROM price_feeds 
WHERE created_at > NOW() - INTERVAL '1 minute'
ORDER BY created_at DESC;
```

## 🎯 Step 5: Advanced Testing

### Load Testing
```bash
# Install Apache Bench for load testing
brew install httpie

# Test API under load
for i in {1..100}; do
  curl -s http://localhost:3000/api/v1/price/BTC/USD &
done
wait

# Check response times
curl -w "@curl-format.txt" -o /dev/null -s http://localhost:3000/api/v1/price/BTC/USD
```

### Create curl format file
```bash
cat > curl-format.txt << 'EOF'
     time_namelookup:  %{time_namelookup}\n
        time_connect:  %{time_connect}\n
     time_appconnect:  %{time_appconnect}\n
    time_pretransfer:  %{time_pretransfer}\n
       time_redirect:  %{time_redirect}\n
  time_starttransfer:  %{time_starttransfer}\n
                     ----------\n
          time_total:  %{time_total}\n
EOF
```

### Manipulation Testing
```bash
# This would require actual oracle manipulation, which we can't do in testing
# But you can check the manipulation detection by viewing logs:
docker-compose logs oracle-backend | grep -i "manipulation"
```

## 📊 Step 6: Monitoring & Metrics

### Prometheus Metrics
```bash
# Start monitoring stack
docker-compose up -d prometheus grafana

# Access Prometheus
open http://localhost:9090

# Access Grafana
open http://localhost:3002
# Login: admin/goquant123
```

### PgAdmin for Database Management
```bash
# Start PgAdmin (development profile)
docker-compose --profile development up -d pgadmin

# Access PgAdmin
open http://localhost:5050
# Login: admin@goquant.io / goquant123

# Add server connection:
# Host: postgres
# Port: 5432
# Database: goquant
# Username: postgres
# Password: password
```

## 🚨 Troubleshooting

### Common Issues

1. **Docker not running:**
   ```bash
   # Start Docker Desktop app
   open -a Docker
   ```

2. **Port conflicts:**
   ```bash
   # Check what's using ports
   lsof -i :3000
   lsof -i :5432
   lsof -i :6379
   
   # Kill processes if needed
   sudo kill -9 <PID>
   ```

3. **Database connection issues:**
   ```bash
   # Reset database
   docker-compose down postgres
   docker volume rm goquant-oracle-system_postgres_data
   docker-compose up -d postgres
   ```

4. **Oracle API errors:**
   ```bash
   # Check if Solana RPC is accessible
   curl https://api.mainnet-beta.solana.com -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
   ```

### Debug Logs
```bash
# Enable debug logging
export RUST_LOG=debug

# View detailed logs
docker-compose logs -f oracle-backend

# Filter for specific issues
docker-compose logs oracle-backend | grep -E "(ERROR|WARN|manipulation|price)"
```

## ✅ Expected Test Results

### Successful Setup Indicators:
- [ ] All Docker containers running healthy
- [ ] Database tables created with sample data
- [ ] API endpoints returning valid JSON responses
- [ ] WebSocket streaming price updates
- [ ] Price data being stored in PostgreSQL
- [ ] Redis caching working (sub-50ms response times)
- [ ] Prometheus metrics collecting data
- [ ] No error logs in backend service

### Performance Targets:
- [ ] API response time < 50ms
- [ ] Price update latency < 500ms  
- [ ] WebSocket latency < 100ms
- [ ] 99.9% uptime during testing period

## 🔧 Manual Testing Scenarios

### Test Oracle Failover
1. Stop one oracle source (simulate by modifying code temporarily)
2. Verify system continues with remaining oracle
3. Check manipulation detection flags the issue

### Test Price Deviation Detection
1. Monitor current BTC price
2. Look for natural price movements >5%
3. Verify alerts are generated in logs/database

### Test Database Performance
```sql
-- Run performance test queries
SELECT COUNT(*) FROM price_feeds;
EXPLAIN ANALYZE SELECT * FROM latest_prices;
EXPLAIN ANALYZE SELECT * FROM oracle_health;
```

This comprehensive testing guide will help you verify all components of the GoQuant Oracle system are working correctly!
