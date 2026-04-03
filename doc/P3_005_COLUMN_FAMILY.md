# P3-005: Column Family Support

## Overview

This module provides column family isolation for tokitai-context, allowing multiple logical key-value stores within a single physical storage system. Column families enable data isolation, independent configurations, and organized data management.

## Features

- **Isolated Column Families**: Each family has independent storage and configuration
- **Batch Operations**: Efficient batch put/delete within and across families
- **Statistics & Metrics**: Per-family and aggregate statistics
- **Prometheus Export**: Built-in metrics export for monitoring
- **Async/Await API**: Non-blocking operations for high performance

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                  ColumnFamilyManager                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  families: DashMap<String, Arc<ColumnFamily>>        │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   default    │  │    users     │  │   sessions   │      │
│  │   family     │  │    family    │  │    family    │      │
│  │              │  │              │  │              │      │
│  │ - storage    │  │ - storage    │  │ - storage    │      │
│  │ - stats      │  │ - stats      │  │ - stats      │      │
│  │ - config     │  │ - config     │  │ - config     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└──────────────────────────────────────────────────────────────┘
```

## Quick Start

### Basic Usage

```rust
use tokitai_context::column_family::{ColumnFamilyManager, ColumnFamilyConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create manager
    let manager = ColumnFamilyManager::new();
    manager.init().await?;
    
    // Get default family
    let default_family = manager.default_family()?;
    
    // Put and get data
    default_family.put(b"key1", b"value1".to_vec()).await?;
    let value = default_family.get(b"key1").await?;
    
    println!("Value: {:?}", value);
    Ok(())
}
```

### Creating Column Families

```rust
// Create with custom configuration
let config = ColumnFamilyConfig::new()
    .with_max_size(1024 * 1024 * 1024)  // 1GB
    .with_block_cache_size(64 * 1024 * 1024)  // 64MB
    .with_bloom_filter(true)
    .with_compression(CompressionType::Lz4);

manager.create_family("users", config)?;
manager.create_family("sessions", ColumnFamilyConfig::default())?;
manager.create_family("cache", ColumnFamilyConfig::default())?;
```

### Data Isolation

```rust
let users = manager.get_family("users")?;
let sessions = manager.get_family("sessions")?;

// Data in different families is isolated
users.put(b"user:1", b"alice".to_vec()).await?;
sessions.put(b"session:1", b"session_data".to_vec()).await?;

// Users family cannot see sessions data
assert_eq!(users.get(b"session:1").await?, None);

// Sessions family cannot see users data
assert_eq!(sessions.get(b"user:1").await?, None);
```

## API Reference

### ColumnFamilyManager

#### Creating and Managing Families

```rust
// Create new manager
let manager = ColumnFamilyManager::new();

// Or with custom root path
let manager = ColumnFamilyManager::with_root_path("/path/to/data");

// Initialize (creates default family)
manager.init().await?;

// Create a new family
manager.create_family("my_family", ColumnFamilyConfig::default())?;

// Get a family
let family = manager.get_family("my_family")?;

// List all families
let families = manager.list_families();

// Check if family exists
if manager.has_family("my_family") {
    println!("Family exists!");
}

// Drop a family (cannot drop default)
manager.drop_family("my_family")?;
```

#### Batch Operations

```rust
// Single family batch
let mut batch = BatchOperation::new("users".to_string());
batch.put(b"user:1".to_vec(), b"alice".to_vec());
batch.put(b"user:2".to_vec(), b"bob".to_vec());
batch.delete(b"user:3".to_vec());

manager.batch(batch).await?;

// Multi-family batch
let batch1 = BatchOperation::new("users".to_string());
let batch2 = BatchOperation::new("sessions".to_string());

manager.batch_multi(vec![batch1, batch2]).await?;
```

### ColumnFamily

#### Basic Operations

```rust
let family = manager.get_family("users")?;

// Put
family.put(b"key", b"value".to_vec()).await?;

// Get
let value = family.get(b"key").await?;

// Delete
family.delete(b"key").await?;

// Exists
if family.exists(b"key").await? {
    println!("Key exists!");
}
```

#### Iteration

```rust
// Get all keys
let keys = family.keys().await?;

// Get all key-value pairs
let entries = family.iter().await?;
for (key, value) in entries {
    println!("{}: {:?}", hex::encode(&key), value);
}
```

#### Batch Operations

```rust
// Batch put
let entries = vec![
    (b"key1".to_vec(), b"value1".to_vec()),
    (b"key2".to_vec(), b"value2".to_vec()),
];
family.batch_put(entries).await?;

// Batch delete
let keys = vec![b"key1".to_vec(), b"key2".to_vec()];
family.batch_delete(keys).await?;
```

#### Utilities

```rust
// Clear all data
family.clear().await?;

// Get estimated size
let size = family.estimated_size().await;

// Get statistics
let stats = family.stats();
println!("Total puts: {}", stats.total_puts.load(Ordering::Relaxed));
```

## Configuration

### ColumnFamilyConfig Options

```rust
let config = ColumnFamilyConfig {
    // Maximum size before compaction (default: 1GB)
    max_size: 1024 * 1024 * 1024,
    
    // Block cache size (default: 64MB)
    block_cache_size: 64 * 1024 * 1024,
    
    // Enable bloom filter (default: true)
    enable_bloom_filter: true,
    
    // Compression algorithm (default: None)
    compression: CompressionType::Lz4,
    
    // Write buffer size (default: 64MB)
    write_buffer_size: 64 * 1024 * 1024,
    
    // Number of LSM levels (default: 7)
    num_levels: 7,
};
```

### Compression Types

```rust
enum CompressionType {
    None,   // No compression
    Snappy, // Fast compression
    Zlib,   // Standard compression
    Bz2,    // High compression
    Lz4,    // Very fast compression
    Zstd,   // Modern high-performance compression
}
```

## Statistics and Monitoring

### ColumnFamilyStats

```rust
let stats = family.stats();

// Operation counts
let puts = stats.total_puts.load(Ordering::Relaxed);
let gets = stats.total_gets.load(Ordering::Relaxed);
let deletes = stats.total_deletes.load(Ordering::Relaxed);

// Bytes transferred
let written = stats.total_bytes_written.load(Ordering::Relaxed);
let read = stats.total_bytes_read.load(Ordering::Relaxed);

// Size info
let size = stats.estimated_size.load(Ordering::Relaxed);
let files = stats.num_files.load(Ordering::Relaxed);

// Cache performance
let hit_rate = stats.cache_hit_rate();  // 0.0 to 1.0
```

### Prometheus Metrics

```rust
// Single family metrics
let metrics = family.to_prometheus();
println!("{}", metrics);

// Example output:
// tokitai_column_family_puts_total{family="users"} 100
// tokitai_column_family_gets_total{family="users"} 500
// tokitai_column_family_deletes_total{family="users"} 50
// tokitai_column_family_bytes_written_total{family="users"} 10240
// tokitai_column_family_bytes_read_total{family="users"} 51200
// tokitai_column_family_estimated_size_bytes{family="users"} 1048576
// tokitai_column_family_num_files{family="users"} 3
// tokitai_column_family_cache_hit_rate{family="users"} 0.9500

// All families metrics
let all_metrics = manager.to_prometheus();
```

### Total Statistics

```rust
// Get aggregate stats across all families
let total = manager.total_stats();
println!("Total puts across all families: {}", total.total_puts.load(Ordering::Relaxed));
```

## Use Cases

### Session Storage

```rust
manager.create_family("sessions", ColumnFamilyConfig::default())?;
let sessions = manager.get_family("sessions")?;

// Store session data
sessions.put(
    b"session:abc123",
    b"user_id=123&expires=1234567890".to_vec()
).await?;
```

### User Data Isolation

```rust
manager.create_family("users", ColumnFamilyConfig::default())?;
manager.create_family("user_preferences", ColumnFamilyConfig::default())?;

let users = manager.get_family("users")?;
let prefs = manager.get_family("user_preferences")?;

// Store in separate families
users.put(b"user:1", b"alice".to_vec()).await?;
prefs.put(b"prefs:1", b"theme=dark".to_vec()).await?;
```

### Cache Layer

```rust
let cache_config = ColumnFamilyConfig::new()
    .with_block_cache_size(128 * 1024 * 1024)
    .with_max_size(2 * 1024 * 1024 * 1024);

manager.create_family("cache", cache_config)?;
let cache = manager.get_family("cache")?;

// Use for caching
cache.put(b"cache:key", b"cached_value".to_vec()).await?;
```

## Error Handling

### ColumnFamilyError Types

```rust
pub enum ColumnFamilyError {
    NotFound(String),           // Family doesn't exist
    AlreadyExists(String),      // Family already exists
    InvalidName(String),        // Invalid family name
    Storage(String),            // Storage operation failed
    Io(std::io::Error),         // IO error
    Serialization(String),      // Serialization failed
    BatchFailed(String),        // Batch operation failed
}
```

### Handling Errors

```rust
match manager.get_family("nonexistent") {
    Ok(family) => {
        // Use family
    }
    Err(ColumnFamilyError::NotFound(name)) => {
        eprintln!("Family {} not found, creating...", name);
        manager.create_family(&name, ColumnFamilyConfig::default())?;
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Testing

### Unit Tests

Run unit tests:

```bash
cargo test --lib column_family::tests
```

### Example Test

```rust
#[tokio::test]
async fn test_column_family_operations() {
    let temp_dir = TempDir::new().unwrap();
    let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
    manager.init().await.unwrap();

    let family = manager.get_family("default").unwrap();
    family.put(b"key", b"value".to_vec()).await.unwrap();

    let value = family.get(b"key").await.unwrap();
    assert_eq!(value, Some(b"value".to_vec()));
}
```

## Performance Considerations

### Memory Usage

Each column family maintains:
- In-memory storage (HashMap)
- Statistics counters
- Configuration

For large datasets, consider:
- Using appropriate `block_cache_size`
- Setting reasonable `max_size` limits
- Monitoring `estimated_size`

### Batch Operations

Batch operations are more efficient than individual operations:

```rust
// Less efficient
for i in 0..1000 {
    family.put(&key[i], value[i].clone()).await?;
}

// More efficient
let entries: Vec<_> = (0..1000)
    .map(|i| (key[i].clone(), value[i].clone()))
    .collect();
family.batch_put(entries).await?;
```

### Cache Hit Rate

Monitor cache hit rate for performance:

```rust
let hit_rate = family.stats().cache_hit_rate();
if hit_rate < 0.5 {
    // Consider increasing block_cache_size
    warn!("Low cache hit rate: {}", hit_rate);
}
```

## Best Practices

### Naming Conventions

```rust
// Good names
manager.create_family("users", config)?;
manager.create_family("user_sessions", config)?;
manager.create_family("cache-lru", config)?;

// Bad names (will fail)
manager.create_family("", config)?;  // Empty
manager.create_family("users@data", config)?;  // Invalid chars
```

### Family Organization

```rust
// Organize by data type
manager.create_family("users", config)?;
manager.create_family("products", config)?;
manager.create_family("orders", config)?;

// Or by access pattern
manager.create_family("hot_data", config)?;
manager.create_family("cold_data", config)?;
manager.create_family("archive", config)?;
```

### Resource Cleanup

```rust
// Drop unused families
manager.drop_family("temp_data")?;

// Clear data before dropping
family.clear().await?;
manager.drop_family("cleanup_target")?;
```

## Limitations

1. **In-Memory Storage**: Current implementation uses in-memory HashMap. For persistence, integrate with FileKV backend.

2. **No Transactions**: Cross-family operations are not atomic.

3. **No Replication**: Data is not replicated across nodes. Use with distributed coordination for HA.

## Future Enhancements

- [ ] Persistent storage backend integration
- [ ] Cross-family transactions
- [ ] Compaction strategies
- [ ] SST file format support
- [ ] Range queries and iterators
- [ ] TTL (Time-To-Live) support
- [ ] Column family snapshots

## Related Modules

- **P3-004**: Distributed Coordination - For multi-node deployments
- **P3-001**: Async I/O - For non-blocking operations
- **P2-004**: Block Cache - For caching optimization

## References

- [RocksDB Column Families](https://github.com/facebook/rocksdb/wiki/Column-Families)
- [LevelDB Architecture](https://github.com/google/leveldb/blob/main/doc/impl.md)
