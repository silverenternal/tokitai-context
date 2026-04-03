# Tracing Event Classification

## Overview

The tracing event classification system provides structured logging for the tokitai-context crate. All tracing events are categorized by component and severity level, enabling fine-grained filtering in production environments.

## Tracing Targets

The system defines 10 tracing targets, each representing a different subsystem:

| Target | Default Level | Description |
|--------|--------------|-------------|
| `tokitai::storage` | INFO | FileKV operations (put, get, delete, compaction) |
| `tokitai::merge` | INFO | Merge operations (segment merge, compaction) |
| `tokitai::cache` | WARN | Cache operations (block cache, bloom filter, ARC) |
| `tokitai::wal` | INFO | Write-Ahead Log operations |
| `tokitai::index` | INFO | Index operations (sparse index, hash index) |
| `tokitai::branch` | INFO | Branch management operations |
| `tokitai::facade` | INFO | High-level API calls (Context facade) |
| `tokitai::error` | ERROR | Error events (always logged) |
| `tokitai::metrics` | INFO | Performance metrics and statistics |
| `tokitai::general` | INFO | General application events |

## Usage

### Basic Initialization

```rust
use tokitai_context::tracing_config::init_tracing;

fn main() -> anyhow::Result<()> {
    // Initialize with default filters
    init_tracing(None)?;
    
    // Your application code...
    Ok(())
}
```

### Custom Filter Configuration

```rust
use tokitai_context::tracing_config::init_tracing;

fn main() -> anyhow::Result<()> {
    // Enable debug logging for storage, info for merge, warn for cache
    let env_filter = "tokitai::storage=debug,tokitai::merge=info,tokitai::cache=warn";
    init_tracing(Some(env_filter))?;
    
    // Your application code...
    Ok(())
}
```

### Minimal Production Configuration

```rust
use tokitai_context::tracing_config::init_tracing_minimal;

fn main() -> anyhow::Result<()> {
    // Only log warnings for storage, errors, and info for facade
    init_tracing_minimal()?;
    
    // Your application code...
    Ok(())
}
```

### JSON Output for Log Aggregation

```rust
use tokitai_context::tracing_config::init_tracing_json;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // Initialize JSON logging for ELK stack, Splunk, etc.
    init_tracing_json("./logs")?;
    
    // Your application code...
    Ok(())
}
```

## Environment Variable Control

Tracing levels can also be controlled via the `RUST_LOG` environment variable:

```bash
# Debug logging for all tokitai components
RUST_LOG=tokitai=debug cargo run

# Only storage and error events
RUST_LOG=tokitai::storage=debug,tokitai::error=error cargo run

# Production: warnings and errors only
RUST_LOG=tokitai=warn cargo run
```

## Output Format

### Console Output (stdout)

```
thread-id thread-name LEVEL target -> file:line Message
1234 main INFO tokitai::general -> src/lib.rs:45 Tracing initialized
1234 worker-1 DEBUG tokitai::storage -> src/file_kv/mod.rs:123 PUT key=abc123 size=1024
1234 worker-2 WARN tokitai::cache -> src/block_cache.rs:234 Cache miss rate > 50%
```

### File Output (rolling daily files)

Log files are stored in `./logs/` with daily rotation:
- `tokitai.log.YYYY-MM-DD` - Current day's logs
- `tokitai.log.YYYY-MM-DD.gz` - Compressed historical logs

## Structured Fields

All tracing events include structured fields for easy querying:

```rust
// Storage operations
tracing::debug!(
    target: TracingTarget::Storage.as_str(),
    key = %hash,
    size = data.len(),
    layer = ?layer,
    "Stored content"
);

// Cache events
tracing::warn!(
    target: TracingTarget::Cache.as_str(),
    hit_rate = 0.42,
    threshold = 0.5,
    "Cache miss rate exceeds threshold"
);

// Error events
tracing::error!(
    target: TracingTarget::Error.as_str(),
    error = %e,
    operation = "compaction",
    segment_id = segment_id,
    "Compaction failed"
);
```

## Performance Considerations

- **Async Logging**: File writes are buffered and flushed asynchronously
- **Sampling**: For high-volume events, consider implementing sampling
- **Level Filtering**: Use appropriate levels to reduce overhead in production
- **JSON Overhead**: JSON formatting adds ~10-20% overhead vs text format

## Best Practices

1. **Use Appropriate Levels**:
   - `ERROR`: Actual errors that require attention
   - `WARN`: Recoverable issues or degraded performance
   - `INFO`: Normal operational events
   - `DEBUG`: Detailed information for troubleshooting

2. **Include Context**: Always include relevant structured fields (key, size, duration, etc.)

3. **Avoid Sensitive Data**: Never log secrets, tokens, or PII

4. **Consistent Formatting**: Use the target constants from `TracingTarget` enum

5. **Test with Filters**: Verify behavior with different filter configurations

## Migration Guide

### From println!/eprintln!

```rust
// Before
println!("Stored content: {}", hash);

// After
tracing::info!(target: "tokitai::storage", hash = %hash, "Stored content");
```

### From tracing without targets

```rust
// Before
tracing::info!("Cache hit for key {}", key);

// After
tracing::info!(target: "tokitai::cache", key = %key, "Cache hit");
```

## Troubleshooting

### No Logs Appearing

1. Check that `init_tracing()` is called before any tracing events
2. Verify the filter configuration includes your target
3. Ensure the log level is appropriate (e.g., INFO vs DEBUG)

### Too Much Log Noise

1. Increase the filter level (e.g., `debug` → `info` → `warn`)
2. Filter to specific targets (e.g., only `tokitai::error`)
3. Use `init_tracing_minimal()` for production

### File Logs Not Rotating

1. Check write permissions in the log directory
2. Verify disk space availability
3. Review file retention policy (default: 7 days)

## Future Enhancements

- [ ] Log sampling for high-volume events
- [ ] Metrics export to Prometheus
- [ ] Distributed tracing with OpenTelemetry
- [ ] Real-time log streaming API
