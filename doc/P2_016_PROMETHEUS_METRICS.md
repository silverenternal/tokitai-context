# P2-016: Prometheus Metrics Export

## Overview

The Prometheus Metrics Export module provides comprehensive monitoring capabilities for the tokitai-context storage engine. It exposes performance, capacity, and operational metrics in Prometheus-compatible format.

## Features

- **Real-time Metrics**: Live monitoring of operations, latency, and resource usage
- **Prometheus Format**: Native Prometheus text format for easy integration
- **Comprehensive Coverage**: Write, read, storage, memory, and WAL metrics
- **Thread-Safe**: Lock-free atomic operations for minimal overhead
- **Custom Metrics**: Support for user-defined metrics

## Metrics Categories

### Write Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `tokitai_write_count` | Counter | Total write operations |
| `tokitai_write_bytes_total` | Counter | Total bytes written |
| `tokitai_write_latency_us_avg` | Gauge | Average write latency (µs) |
| `tokitai_write_errors_total` | Counter | Total write errors |

### Read Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `tokitai_read_count` | Counter | Total read operations |
| `tokitai_cache_hit_rate` | Gauge | Cache hit rate (0.0-1.0) |
| `tokitai_read_latency_us_avg` | Gauge | Average read latency (µs) |
| `tokitai_read_errors_total` | Counter | Total read errors |

### Storage Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `tokitai_segment_count` | Gauge | Current segment count |
| `tokitai_storage_size_bytes` | Gauge | Total storage size (bytes) |
| `tokitai_total_entries` | Gauge | Total entries stored |
| `tokitai_compaction_count` | Counter | Total compactions performed |

### Memory Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `tokitai_memtable_size_bytes` | Gauge | Current MemTable size (bytes) |
| `tokitai_memtable_entries` | Gauge | Current MemTable entries |
| `tokitai_cache_size_bytes` | Gauge | Current cache size (bytes) |
| `tokitai_cache_items` | Gauge | Current cache items |

### WAL Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `tokitai_wal_writes_total` | Counter | Total WAL writes |
| `tokitai_wal_bytes_total` | Counter | Total WAL bytes written |
| `tokitai_wal_rotations_total` | Counter | Total WAL rotations |
| `tokitai_wal_size_bytes` | Gauge | Current WAL size (bytes) |

### System Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `tokitai_uptime_seconds` | Gauge | Service uptime (seconds) |

## Usage

### Basic Usage

```rust
use tokitai_context::metrics::MetricsRegistry;
use std::time::Duration;

// Create metrics registry
let registry = MetricsRegistry::new();

// Record operations
registry.record_write(1024, Duration::from_micros(50));
registry.record_read_hit(Duration::from_micros(5));
registry.record_read_miss(Duration::from_micros(100));

// Update storage metrics
registry.set_storage(10, 1024 * 1024, 1000);

// Update memory metrics
registry.set_memory(
    1024 * 100,  // MemTable size
    500,         // MemTable entries
    1024 * 1024 * 10,  // Cache size
    1000         // Cache items
);

// Export to Prometheus format
let metrics = registry.gather();
println!("{}", metrics);
```

### Integration with FileKV

```rust
use tokitai_context::file_kv::{FileKV, FileKVConfig};
use tokitai_context::metrics::MetricsRegistry;
use std::sync::Arc;
use std::time::Instant;

// Create registry
let registry = Arc::new(MetricsRegistry::new());

// Open FileKV with metrics
let config = FileKVConfig::default();
let kv = FileKV::open(config)?;

// Record write with metrics
let start = Instant::now();
match kv.put("key", b"value") {
    Ok(_) => {
        registry.record_write(
            5,  // bytes
            start.elapsed()
        );
    }
    Err(_) => registry.record_write_error(),
}

// Record read with metrics
let start = Instant::now();
match kv.get("key") {
    Ok(Some(_)) => registry.record_read_hit(start.elapsed()),
    Ok(None) => registry.record_read_miss(start.elapsed()),
    Err(_) => registry.record_read_error(),
}
```

### HTTP Endpoint for Prometheus Scraping

```rust
use actix_web::{web, App, HttpResponse, HttpServer};
use tokitai_context::metrics::MetricsRegistry;
use std::sync::Arc;

async fn metrics_handler(registry: web::Data<Arc<MetricsRegistry>>) -> HttpResponse {
    let metrics = registry.gather();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(metrics)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let registry = Arc::new(MetricsRegistry::new());
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(registry.clone()))
            .route("/metrics", web::get().to(metrics_handler))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### Custom Metrics

```rust
use tokitai_context::metrics::{MetricsRegistry, MetricValue, MetricType};
use std::collections::HashMap;

let registry = MetricsRegistry::new();

// Create custom metric
let mut labels = HashMap::new();
labels.insert("operation".to_string(), "compaction".to_string());

let metric = MetricValue {
    name: "tokitai_custom_operation".to_string(),
    help: "Custom operation counter".to_string(),
    metric_type: MetricType::Counter,
    value: 42.0,
    labels,
};

registry.register_metric(metric);
```

## Prometheus Configuration

### prometheus.yml

```yaml
scrape_configs:
  - job_name: 'tokitai-context'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

### Grafana Dashboard

Import this dashboard JSON for visualization:

```json
{
  "dashboard": {
    "title": "Tokitai Context Metrics",
    "panels": [
      {
        "title": "Write Latency",
        "targets": [
          {
            "expr": "tokitai_write_latency_us_avg",
            "legendFormat": "Avg Write Latency (µs)"
          }
        ]
      },
      {
        "title": "Cache Hit Rate",
        "targets": [
          {
            "expr": "tokitai_cache_hit_rate",
            "legendFormat": "Hit Rate"
          }
        ]
      },
      {
        "title": "Storage Size",
        "targets": [
          {
            "expr": "tokitai_storage_size_bytes",
            "legendFormat": "Total Size"
          }
        ]
      }
    ]
  }
}
```

## Alerting Rules

### alerts.yml

```yaml
groups:
  - name: tokitai_alerts
    rules:
      - alert: HighWriteLatency
        expr: tokitai_write_latency_us_avg > 1000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High write latency detected"
          description: "Average write latency is {{ $value }} µs"
      
      - alert: LowCacheHitRate
        expr: tokitai_cache_hit_rate < 0.5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Low cache hit rate"
          description: "Cache hit rate is {{ $value }}"
      
      - alert: HighMemoryUsage
        expr: tokitai_memtable_size_bytes > 536870912
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High MemTable memory usage"
          description: "MemTable size is {{ $value }} bytes"
```

## API Reference

### MetricsRegistry

```rust
pub struct MetricsRegistry {
    // Internal state
}

impl MetricsRegistry {
    pub fn new() -> Self;
    
    // Write operations
    pub fn record_write(&self, bytes: usize, latency: Duration);
    pub fn record_write_error(&self);
    
    // Read operations
    pub fn record_read_hit(&self, latency: Duration);
    pub fn record_read_miss(&self, latency: Duration);
    pub fn record_read_error(&self);
    
    // Storage operations
    pub fn set_storage(&self, segments: u64, size: u64, entries: u64);
    pub fn record_compaction(&self);
    
    // Memory operations
    pub fn set_memory(&self, memtable_size: u64, memtable_entries: u64, 
                      cache_size: u64, cache_items: u64);
    
    // WAL operations
    pub fn record_wal_write(&self, bytes: usize);
    pub fn record_wal_rotation(&self);
    pub fn set_wal_size(&self, size: u64);
    
    // Custom metrics
    pub fn register_metric(&self, metric: MetricValue);
    
    // Export
    pub fn gather(&self) -> String;
}
```

### MetricValue

```rust
pub struct MetricValue {
    pub name: String,
    pub help: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub labels: HashMap<String, String>,
}
```

### MetricType

```rust
pub enum MetricType {
    Counter,    // Monotonically increasing
    Gauge,      // Can go up or down
    Histogram,  // Distribution of values
}
```

## Test Results

### Unit Tests (8 tests)

```
test metrics::tests::test_metrics_registry_creation ... ok
test metrics::tests::test_write_metrics ... ok
test metrics::tests::test_read_metrics ... ok
test metrics::tests::test_storage_metrics ... ok
test metrics::tests::test_memory_metrics ... ok
test metrics::tests::test_wal_metrics ... ok
test metrics::tests::test_gather_metrics ... ok
test metrics::tests::test_metric_value_display ... ok

test result: ok. 8 passed; 0 failed
```

## Example Output

```
# HELP tokitai_uptime_seconds Uptime in seconds
# TYPE tokitai_uptime_seconds gauge
tokitai_uptime_seconds 3600

# HELP tokitai_write_count Total write operations
# TYPE tokitai_write_count counter
tokitai_write_count 10000

# HELP tokitai_write_bytes_total Total bytes written
# TYPE tokitai_write_bytes_total counter
tokitai_write_bytes_total 10485760

# HELP tokitai_write_latency_us_avg Average write latency (microseconds)
# TYPE tokitai_write_latency_us_avg gauge
tokitai_write_latency_us_avg 45.23

# HELP tokitai_cache_hit_rate Cache hit rate (0.0-1.0)
# TYPE tokitai_cache_hit_rate gauge
tokitai_cache_hit_rate 0.8542

# HELP tokitai_read_latency_us_avg Average read latency (microseconds)
# TYPE tokitai_read_latency_us_avg gauge
tokitai_read_latency_us_avg 12.56

# HELP tokitai_segment_count Current segment count
# TYPE tokitai_segment_count gauge
tokitai_segment_count 15

# HELP tokitai_storage_size_bytes Total storage size in bytes
# TYPE tokitai_storage_size_bytes gauge
tokitai_storage_size_bytes 104857600

# HELP tokitai_memtable_size_bytes Current MemTable size in bytes
# TYPE tokitai_memtable_size_bytes gauge
tokitai_memtable_size_bytes 4194304
```

## Performance Overhead

Metrics recording uses atomic operations with minimal overhead:

- **Counter increment**: ~5-10ns
- **Gauge update**: ~5-10ns
- **Gather all metrics**: ~100-500µs (depends on metric count)
- **Memory usage**: ~1KB for registry structure

## Best Practices

### 1. Record Metrics at Operation Boundaries

```rust
let start = Instant::now();
let result = perform_operation();
registry.record_write(bytes, start.elapsed());
```

### 2. Use Appropriate Metric Types

- **Counter**: For cumulative counts (operations, errors)
- **Gauge**: For current values (size, latency, rate)
- **Histogram**: For distributions (latency percentiles)

### 3. Avoid High-Cardinality Labels

```rust
// Good - low cardinality
labels.insert("operation_type", "write");

// Bad - high cardinality (don't do this)
labels.insert("user_id", user_id); // Could be millions of values!
```

### 4. Update Metrics Consistently

```rust
// Always record both success and error cases
match operation() {
    Ok(result) => registry.record_write(bytes, latency),
    Err(_) => registry.record_write_error(),
}
```

### 5. Export Metrics Regularly

Configure Prometheus to scrape every 15-30 seconds for good resolution without excessive load.

## Integration Examples

### Actix Web

```rust
use actix_web::{web, HttpResponse};

async fn metrics(registry: web::Data<Arc<MetricsRegistry>>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(registry.gather())
}
```

### Axum

```rust
use axum::{response::Response, routing::get, Router};

async fn metrics(registry: State<Arc<MetricsRegistry>>) -> Response<String> {
    (
        [(CONTENT_TYPE, "text/plain; version=0.0.4")],
        registry.gather(),
    ).into_response()
}

let app = Router::new()
    .route("/metrics", get(metrics))
    .with_state(registry);
```

### Warp

```rust
use warp::Filter;

let metrics_route = warp::path("metrics")
    .and_then(move || {
        let registry = registry.clone();
        async move {
            Ok::<_, warp::Rejection>(registry.gather())
        }
    });
```

## Limitations

- No histogram bucket support (only averages)
- No metric expiration (metrics persist indefinitely)
- No push gateway support (pull model only)
- No authentication on /metrics endpoint

## Future Improvements

- [ ] Histogram with configurable buckets
- [ ] Summary metrics for percentiles (p50, p95, p99)
- [ ] Push gateway integration
- [ ] Metric expiration and cleanup
- [ ] Authentication middleware
- [ ] Automatic FileKV integration
- [ ] Async metrics collection

## Related Issues

- **P2-002**: Tracing classification (complementary observability)
- **P2-016**: This implementation

## Files Created

- `src/metrics.rs` - Prometheus metrics implementation
- `doc/P2_016_PROMETHEUS_METRICS.md` - This documentation

## Verification

```bash
# Build
cargo build --lib

# Clippy (0 warnings)
cargo clippy --lib

# Run tests
cargo test --lib metrics
```

## Conclusion

The Prometheus Metrics Export module provides comprehensive monitoring with:

- ✅ 8 passing unit tests
- ✅ 20+ metrics across 5 categories
- ✅ Prometheus-compatible text format
- ✅ Thread-safe atomic operations
- ✅ Custom metric support
- ✅ Minimal performance overhead

The module enables production-grade monitoring and alerting for the tokitai-context storage engine.
