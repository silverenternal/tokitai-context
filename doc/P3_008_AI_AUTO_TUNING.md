# P3-008: AI Auto-tuning

## Overview

The AI Auto-tuning module provides intelligent, automated performance optimization for the tokitai-context storage engine. It uses machine learning-inspired techniques to analyze workload patterns, detect anomalies, and recommend optimal configuration parameters.

## Features

### Metrics Collection
- **System Metrics**: CPU, memory, disk I/O, network
- **Storage Metrics**: Read/write ops, latency, cache hit ratio, compaction
- **Query Metrics**: QPS, latency percentiles, error rate, active queries
- **Time-series Storage**: Efficient metrics history with configurable window

### Workload Analysis
- **Pattern Recognition**: Automatically detect workload patterns
  - Read-heavy, Write-heavy, Mixed
  - Batch load, Analytical, Point lookup, Range scan
  - Idle detection
- **Characteristics Analysis**:
  - Read/write ratio
  - Access randomness
  - Hot key concentration
  - Temporal locality
  - Write burstiness

### Automatic Tuning
- **Tuning Targets**:
  - Throughput optimization
  - Latency optimization
  - Balanced optimization
  - Memory optimization
  - Disk optimization
- **Tunable Parameters**:
  - Block cache size
  - Memtable size
  - Write buffer configuration
  - Compaction settings
  - Parallelism settings

### Anomaly Detection
- **Latency Spikes**: Detect sudden performance degradation
- **Throughput Drops**: Identify capacity issues
- **Memory Pressure**: Alert on high memory usage
- **Write Stalls**: Detect compaction falling behind
- **Error Rate Increases**: Monitor query failures

### Recommendations
- **Confidence Scoring**: Each recommendation includes confidence level
- **Expected Improvement**: Estimated performance gain
- **Risk Assessment**: Low, Medium, High risk levels
- **Actionable Reasons**: Clear explanation of why change is recommended

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       Auto Tuner                             │
├─────────────────────────────────────────────────────────────┤
│  Metrics Collector → Workload Analyzer → Pattern Detection  │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Anomaly      │  │ Tuning       │  │ Configuration│      │
│  │ Detector     │  │ Recommender  │  │ Manager      │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
├─────────────────────────────────────────────────────────────┤
│                    Parameter Storage                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Current      │  │ History      │  │ Bounds       │      │
│  │ Values       │  │              │  │              │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## Usage

### Basic Auto-tuning

```rust
use tokitai_context::auto_tuner::{
    AutoTuner, AutoTunerConfig, TuningTarget,
    MetricsSnapshot, StorageMetrics,
};

// Configure auto-tuner
let config = AutoTunerConfig {
    tuning_target: TuningTarget::Balanced,
    min_adjustment_interval_secs: 300,
    metrics_interval_secs: 10,
    analysis_window_size: 60,
    auto_adjust: false, // Recommend only
    anomaly_detection: true,
    anomaly_sensitivity: 0.8,
    max_change_percent: 0.2,
    cooldown_secs: 600,
    verbose: false,
};

let tuner = AutoTuner::new(config);

// Start monitoring
tuner.start().await?;

// Record metrics periodically
let mut snapshot = MetricsSnapshot::new();
snapshot.storage.read_ops = 1000;
snapshot.storage.write_ops = 200;
snapshot.storage.cache_hit_ratio = 0.75;
snapshot.storage.read_latency_us = 500.0;

tuner.record_metrics(snapshot);

// Get recommendations
let recommendations = tuner.get_recommendations();
for rec in recommendations {
    println!("Recommendation: {}", rec.parameter);
    println!("  Current: {}", rec.current_value);
    println!("  Recommended: {}", rec.recommended_value);
    println!("  Confidence: {:.0}%", rec.confidence * 100.0);
    println!("  Expected improvement: {:.0}%", rec.expected_improvement * 100.0);
}
```

### Workload Pattern Detection

```rust
use tokitai_context::auto_tuner::{
    AutoTuner, AutoTunerConfig, WorkloadPattern,
};

let tuner = AutoTuner::new(AutoTunerConfig::default());

// After recording metrics...
let pattern = tuner.get_workload_pattern();

match pattern {
    WorkloadPattern::ReadHeavy => {
        println!("Read-heavy workload detected");
        // Optimize for reads: increase cache, reduce compaction
    }
    WorkloadPattern::WriteHeavy => {
        println!("Write-heavy workload detected");
        // Optimize for writes: increase buffers, parallelism
    }
    WorkloadPattern::Mixed => {
        println!("Mixed workload detected");
        // Balance read and write optimization
    }
    _ => {}
}
```

### Anomaly Monitoring

```rust
use tokitai_context::auto_tuner::{
    AutoTuner, AutoTunerConfig, AlertSeverity,
};

let tuner = AutoTuner::new(AutoTunerConfig::default());

// Record metrics and check for anomalies
let anomalies = tuner.get_recent_anomalies();
for anomaly in anomalies {
    match anomaly.severity {
        AlertSeverity::Critical => {
            eprintln!("CRITICAL: {}", anomaly.description);
            eprintln!("Action: {}", anomaly.suggested_action);
        }
        AlertSeverity::Warning => {
            eprintln!("WARNING: {}", anomaly.description);
        }
        AlertSeverity::Info => {
            println!("INFO: {}", anomaly.description);
        }
    }
}
```

### Custom Parameter Bounds

```rust
use tokitai_context::auto_tuner::{
    AutoTuner, AutoTunerConfig, ParamBounds,
};

// Define custom parameter bounds
let bounds = ParamBounds {
    block_cache_size: (128 * 1024 * 1024, 2 * 1024 * 1024 * 1024), // 128MB - 2GB
    memtable_size: (32 * 1024 * 1024, 256 * 1024 * 1024),          // 32MB - 256MB
    max_background_jobs: (4, 12),
    l0_file_count: (4, 12),
    compaction_read_amp: (8, 20),
};

let tuner = AutoTuner::with_bounds(AutoTunerConfig::default(), bounds);
```

### Applying Recommendations

```rust
use tokitai_context::auto_tuner::AutoTuner;

let tuner = AutoTuner::new(AutoTunerConfig::default());

// Get and apply recommendations
let recommendations = tuner.get_recommendations();
for rec in &recommendations {
    if rec.confidence > 0.8 && rec.risk_level == RiskLevel::Low {
        match tuner.apply_recommendation(rec) {
            Ok(_) => println!("Applied: {}", rec.parameter),
            Err(e) => eprintln!("Failed to apply: {}", e),
        }
    }
}
```

## Tunable Parameters

### Memory Parameters
| Parameter | Default | Min | Max | Description |
|-----------|---------|-----|-----|-------------|
| block_cache_size | 256MB | 64MB | 4GB | Cache for data blocks |
| memtable_size | 64MB | 16MB | 512MB | In-memory write buffer |
| write_buffer_size | 64MB | 16MB | 512MB | Total write buffer |
| max_write_buffers | 2 | 2 | 8 | Number of write buffers |

### Compaction Parameters
| Parameter | Default | Min | Max | Description |
|-----------|---------|-----|-----|-------------|
| compaction_read_amp | 10 | 5 | 30 | Read amplitude target |
| compaction_write_amp | 10 | 5 | 30 | Write amplitude target |
| compaction_size_amp | 10 | 5 | 30 | Size amplitude target |
| l0_file_count | 4 | 2 | 16 | L0 file trigger |
| l0_stall | 20 | 10 | 40 | L0 stall trigger |

### Parallelism Parameters
| Parameter | Default | Min | Max | Description |
|-----------|---------|-----|-----|-------------|
| max_background_jobs | 6 | 2 | 16 | Background threads |
| max_subcompactions | 4 | 1 | 8 | Subcompaction threads |

### Rate Limiting Parameters
| Parameter | Default | Description |
|-----------|---------|-------------|
| bytes_per_sync | 1MB | Bytes per sync operation |
| wal_bytes_per_sync | 512KB | WAL sync size |
| delayed_write_rate | 16MB/s | Write rate limit |
| max_write_stall_us | 1s | Max stall duration |

## Workload Patterns

### ReadHeavy
- **Characteristics**: >90% reads, high cache importance
- **Optimization**: Increase block cache, optimize for point lookups

### WriteHeavy
- **Characteristics**: >90% writes, buffer importance
- **Optimization**: Increase write buffers, parallelize compaction

### Mixed
- **Characteristics**: Balanced read/write ratio
- **Optimization**: Balance memory between cache and buffers

### BatchLoad
- **Characteristics**: Sustained high write throughput
- **Optimization**: Maximize write parallelism, defer compaction

### Analytical
- **Characteristics**: Large range scans, sequential access
- **Optimization**: Increase readahead, optimize for sequential I/O

### PointLookup
- **Characteristics**: Random reads, low latency requirement
- **Optimization**: Maximize cache, minimize compaction interference

### RangeScan
- **Characteristics**: Sequential reads, large result sets
- **Optimization**: Increase block size, enable prefetching

## Anomaly Types

### LatencySpike
- **Trigger**: P99 latency > 10ms
- **Severity**: Based on deviation from baseline
- **Action**: Check for hot keys, increase cache

### ThroughputDrop
- **Trigger**: QPS drop > 50%
- **Severity**: Based on drop percentage
- **Action**: Check for resource contention

### MemoryPressure
- **Trigger**: Memory usage > 90%
- **Severity**: Critical
- **Action**: Reduce cache size, flush memtables

### CompactionLag
- **Trigger**: Compaction falling behind writes
- **Severity**: Warning
- **Action**: Increase compaction threads

### WriteStall
- **Trigger**: Write stall detected
- **Severity**: Warning
- **Action**: Increase write buffers

## Performance Considerations

### Metrics Collection Overhead
- Keep collection interval >= 10 seconds
- Use efficient data structures for history
- Limit history window size

### Analysis Frequency
- Minimum adjustment interval: 5 minutes
- Cooldown period after changes: 10 minutes
- Avoid thrashing with frequent changes

### Recommendation Confidence
- High confidence (>0.8): Safe to apply automatically
- Medium confidence (0.5-0.8): Review before applying
- Low confidence (<0.5): Informational only

## Testing

The module includes comprehensive tests:
- Metrics snapshot creation
- Workload pattern detection
- Anomaly detection
- Recommendation generation
- Configuration management

Run tests:
```bash
cargo test --lib auto_tuner::tests
```

## Future Enhancements

1. **Reinforcement Learning**: Learn from applied recommendations
2. **Predictive Tuning**: Anticipate workload changes
3. **Multi-objective Optimization**: Balance competing goals
4. **Workload Forecasting**: Predict future resource needs
5. **Automated A/B Testing**: Test recommendations safely

## Monitoring Dashboard Integration

```rust
// Export metrics for monitoring
use tokitai_context::auto_tuner::AutoTuner;

let tuner = AutoTuner::new(AutoTunerConfig::default());
let stats = tuner.get_stats();

// Prometheus-style metrics
println!("auto_tuner_adjustments_total {}", stats.total_adjustments);
println!("auto_tuner_anomalies_total {}", stats.anomalies_detected);
println!("auto_tuner_recommendations_total {}", stats.recommendations_made);
```

## References

- Automatic Database Tuning: https://www.vldb.org/pvldb/vol13/p2977-pavlo.pdf
- Query Optimization: https://db.cs.berkeley.edu/cs286/papers/cbo-tutorial.pdf
- Workload Analysis: https://www.microsoft.com/en-us/research/workload-analysis/
