# Adaptive Segment Pre-allocation (P2-008)

## Overview

Adaptive segment pre-allocation dynamically adjusts the size of segment file pre-allocation based on observed write patterns, replacing the fixed 16MB pre-allocation strategy.

## Problem

The original implementation used a fixed pre-allocation size (16MB by default):

- **Over-allocation**: Small writes (e.g., 100KB segments) waste disk space with 16MB pre-allocated files
- **Under-allocation**: Large writes cause file fragmentation when segments exceed pre-allocated size
- **No adaptation**: Write patterns vary by workload, but pre-allocation was static

## Solution

The `AdaptivePreallocator` module implements dynamic pre-allocation sizing using:

1. **Exponential Weighted Moving Average (EWMA)**: Smoothly adapts to changing write patterns
2. **Configurable bounds**: Minimum and maximum pre-allocation sizes prevent extreme values
3. **History tracking**: Recent segment sizes inform future pre-allocation decisions

## Algorithm

### EWMA Calculation

```rust
// For the first segment, use actual size directly
if segment_count == 1 {
    ewma_segment_size = actual_size
} else {
    // EWMA with configurable alpha (smoothing factor)
    ewma_segment_size = alpha * avg_recent_sizes 
                      + (1.0 - alpha) * ewma_segment_size
}
```

### Pre-allocation Size Calculation

```rust
// Add 10% buffer to reduce fragmentation
optimal_size = ewma_segment_size * 1.1

// Clamp to configured bounds
preallocate_size = clamp(optimal_size, min_bytes, max_bytes)
```

## Configuration

### AdaptivePreallocatorConfig

| Field | Default | Description |
|-------|---------|-------------|
| `min_preallocate_bytes` | 1MB | Minimum pre-allocation size |
| `max_preallocate_bytes` | 64MB | Maximum pre-allocation size |
| `initial_preallocate_bytes` | 16MB | Initial size before adaptation |
| `ewma_alpha` | 0.3 | Smoothing factor (0.0-1.0) |
| `history_size` | 10 | Number of segments to track |
| `enabled` | true | Enable/disable adaptive mode |

### EWMA Alpha Tuning

- **Low alpha (0.1-0.3)**: Smoother adaptation, less responsive to sudden changes
- **Medium alpha (0.3-0.5)**: Balanced adaptation (default)
- **High alpha (0.5-0.8)**: Fast adaptation, may oscillate with variable workloads

## Usage

### Basic Usage

The adaptive pre-allocator is automatically initialized when opening a `FileKV` instance:

```rust
use tokitai_context::file_kv::{FileKV, FileKVConfig};

let mut config = FileKVConfig::default();
config.segment_preallocate_size = 16 * 1024 * 1024; // Initial size

let kv = FileKV::open(config)?;

// Pre-allocation adapts automatically based on write patterns
kv.put("key1", b"value1")?;
kv.put("key2", b"value2")?;
```

### Getting Statistics

```rust
if let Some(stats) = kv.get_preallocator_stats() {
    println!("Current pre-allocate size: {}", stats.current_preallocate_size);
    println!("Average utilization: {:.2}%", stats.avg_utilization * 100.0);
    println!("Segments tracked: {}", stats.segments_tracked);
}
```

### Disabling Adaptive Pre-allocation

To use fixed pre-allocation (original behavior):

```rust
let mut config = FileKVConfig::default();
config.segment_preallocate_size = 16 * 1024 * 1024; // Fixed 16MB

// Or set to 0 to disable pre-allocation entirely
config.segment_preallocate_size = 0;
```

## Integration Points

### FileKV::open()

Initializes the adaptive pre-allocator with configuration derived from `segment_preallocate_size`:

```rust
let adaptive_preallocator = if config.segment_preallocate_size > 0 {
    let prealloc_config = AdaptivePreallocatorConfig {
        initial_preallocate_bytes: config.segment_preallocate_size,
        ..Default::default()
    };
    Some(Arc::new(AdaptivePreallocator::new(prealloc_config)))
} else {
    None
};
```

### FileKV::flush_memtable()

Uses adaptive pre-allocation size when creating new segments:

```rust
let preallocate_size = self.adaptive_preallocator
    .as_ref()
    .map(|p| p.next_preallocate_size())
    .unwrap_or(self.config.segment_preallocate_size);

let segment = SegmentFile::create(segment_id, &segment_path, preallocate_size)?;

// Record segment creation
if let Some(ref preallocator) = self.adaptive_preallocator {
    preallocator.record_segment_created(preallocate_size);
}
```

### FileKV::record_segment_closed()

Updates the adaptation model when segments are closed:

```rust
pub(crate) fn record_segment_closed(&self, actual_size: u64) {
    if let Some(ref preallocator) = self.adaptive_preallocator {
        preallocator.record_segment_closed(actual_size);
    }
}
```

### Compaction

Compaction also uses adaptive pre-allocation for new segments:

```rust
let preallocate_size = kv.get_next_preallocate_size();
let new_segment = SegmentFile::create(new_segment_id, &temp_segment_path, preallocate_size)?;

// After successful compaction
kv.record_segment_closed(segment_size);
```

## Performance Characteristics

### Memory Overhead

- **Per-instance**: ~100 bytes for configuration and state
- **Per-segment tracked**: 8 bytes (u64 size in history vector)
- **Total**: Negligible (< 1KB for typical workloads)

### CPU Overhead

- **Per-segment creation**: O(1) - simple size lookup
- **Per-segment close**: O(history_size) - EWMA calculation over history
- **Typical cost**: < 1µs per operation

### Disk Space Savings

Expected disk space savings vary by workload:

| Workload Pattern | Fixed 16MB | Adaptive | Savings |
|-----------------|------------|----------|---------|
| Small writes (100KB avg) | 16MB | 110KB | 99% |
| Medium writes (8MB avg) | 16MB | 8.8MB | 45% |
| Large writes (32MB avg) | 16MB | 35.2MB* | -120%* |
| Mixed variable | 16MB | 10-20MB | 25-40% |

*Over-allocation is bounded by `max_preallocate_bytes` (default 64MB)

## Testing

The implementation includes 7 unit tests:

1. **test_config_default**: Validates default configuration values
2. **test_adaptive_preallocator_basic**: Tests basic creation and recording
3. **test_adaptive_preallocator_adaptation**: Verifies size adaptation with growing segments
4. **test_adaptive_preallocator_min_max_bounds**: Tests boundary enforcement
5. **test_adaptive_preallocator_disabled**: Verifies disabled mode behavior
6. **test_adaptive_preallocator_stats**: Tests statistics collection
7. **test_adaptive_preallocator_ewma_smoothing**: Validates EWMA smoothing behavior

Run tests with:

```bash
cargo test --lib adaptive_preallocator
```

## Monitoring

### Key Metrics

- **current_preallocate_size**: Current pre-allocation size being used
- **avg_utilization**: Ratio of actual data to pre-allocated space
  - < 0.5: Over-allocating (wasting space)
  - 0.5-0.9: Good balance
  - > 0.9: May benefit from slightly higher pre-allocation
- **segments_tracked**: Number of segments in history window
- **total_preallocated_bytes**: Cumulative pre-allocated space
- **total_used_bytes**: Cumulative actual data written

### Alerting Recommendations

Consider alerting when:

- `avg_utilization < 0.3` for extended periods (severe over-allocation)
- `avg_utilization > 0.95` consistently (frequent file extension)
- `segments_tracked < history_size` after extended operation (potential bug)

## Future Enhancements

Potential improvements for future iterations:

1. **Workload classification**: Detect sequential vs random write patterns
2. **Time-based adaptation**: Consider time-of-day or day-of-week patterns
3. **Predictive pre-allocation**: Use ML to predict optimal sizes
4. **Per-key-space adaptation**: Different strategies for different data types
5. **Compression-aware**: Adjust based on compression ratio if enabled

## Related Issues

- **P2-008**: Adaptive Segment Pre-allocation (this implementation)
- **P1-001**: Base Performance Optimization (write performance)
- **P3-005**: Compression Dictionary (interacts with segment sizing)

## References

- [LSM-Tree Compaction Strategies](https://en.wikipedia.org/wiki/Log-structured_merge-tree)
- [Exponential Smoothing](https://en.wikipedia.org/wiki/Exponential_smoothing)
- [File Pre-allocation Best Practices](https://www.kernel.org/doc/html/latest/filesystems/ext4/index.html)
