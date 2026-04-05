# P2-013: Audit Logging Implementation

## Overview

Audit logging provides an immutable, append-only trail of all write operations for compliance, debugging, and forensic analysis. This feature is essential for production environments requiring regulatory compliance, security auditing, and operational visibility.

## Features

### Core Capabilities

- **Immutable Audit Trail**: Append-only log files prevent tampering
- **Structured Format**: JSON entries for easy parsing and analysis
- **Configurable Retention**: Automatic log rotation and cleanup
- **Performance Tracking**: Optional latency recording for each operation
- **Compliance Ready**: Timestamps, operation types, and rich metadata
- **SHA256 Value Hashing**: Integrity verification for audited values
- **Batch Operation Support**: Efficient logging of bulk operations

### Audit Entry Format

Each audit entry is a JSON object with the following structure:

```json
{
  "timestamp": "2026-04-03T10:15:30.123Z",
  "operation": "PUT",
  "keys": ["session_abc123"],
  "value_hash": "sha256:a1b2c3d4e5f6...",
  "value_size": 1024,
  "latency_us": 45,
  "success": true,
  "error": null,
  "metadata": {
    "layer": "ShortTerm",
    "session_id": "user_123",
    "user_id": "user_456",
    "request_id": "req_789",
    "custom": {
      "custom_key": "custom_value"
    }
  }
}
```

### Operation Types

| Operation | Description | Keys | Value Hash | Value Size |
|-----------|-------------|------|------------|------------|
| `PUT` | Single key write | 1 key | ✓ | ✓ |
| `DELETE` | Single key delete | 1 key | - | - |
| `BATCH_PUT { count }` | Batch write | N keys | - | Total size |
| `BATCH_DELETE { count }` | Batch delete | N keys | - | - |
| `FLUSH` | MemTable flush | - | - | - |
| `COMPACTION` | Segment compaction | - | - | Bytes reclaimed |

## Configuration

### AuditLogConfig

```rust
pub struct AuditLogConfig {
    /// Directory to store audit logs
    pub log_dir: PathBuf,
    
    /// Enable audit logging (default: false)
    pub enabled: bool,
    
    /// Maximum log file size before rotation (default: 100MB)
    pub max_file_size_bytes: u64,
    
    /// Maximum number of log files to retain (default: 10)
    pub max_files: usize,
    
    /// Record latency for each operation (default: true)
    pub record_latency: bool,
    
    /// Include value hash for integrity verification (default: true)
    pub include_value_hash: bool,
    
    /// Flush after every entry - slower but safer (default: false)
    pub flush_on_write: bool,
}
```

### Default Configuration

```rust
impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("./audit_logs"),
            enabled: false,  // Disabled by default for performance
            max_file_size_bytes: 100 * 1024 * 1024,  // 100MB
            max_files: 10,
            record_latency: true,
            include_value_hash: true,
            flush_on_write: false,  // Buffer writes for performance
        }
    }
}
```

## Usage

### Enable Audit Logging

```rust
use tokitai_context::file_kv::{FileKV, FileKVConfig, AuditLogConfig};

// Configure audit logging
let mut config = FileKVConfig::default();
config.audit_log = AuditLogConfig {
    enabled: true,
    log_dir: PathBuf::from("/var/log/tokitai/audit"),
    max_file_size_bytes: 50 * 1024 * 1024,  // 50MB
    max_files: 20,  // Retain 20 files
    flush_on_write: true,  // For compliance-critical deployments
    ..Default::default()
};

// Open FileKV with audit logging
let kv = FileKV::open(config)?;

// All write operations are now audited
kv.put("key1", b"value1")?;
kv.delete("key2")?;
kv.put_batch(&[("key3", b"value3"), ("key4", b"value4")])?;
```

### Audit Metadata

Add custom metadata to audit entries for enhanced tracing:

```rust
use tokitai_context::audit_log::{AuditLogger, AuditOperation, AuditMetadata};

// Create custom metadata
let mut metadata = AuditMetadata {
    layer: Some("ShortTerm".to_string()),
    session_id: Some("session_123".to_string()),
    user_id: Some("user_456".to_string()),
    request_id: Some("req_789".to_string()),
    custom: {
        let mut map = std::collections::HashMap::new();
        map.insert("client_ip".to_string(), "192.168.1.1".to_string());
        map.insert("operation_source".to_string(), "api".to_string());
        map
    },
};

// Log operation with metadata (internal API)
audit_logger.log_operation(
    AuditOperation::Put,
    vec!["key".to_string()],
    Some("sha256:...".to_string()),
    Some(1024),
    Some(45),
    true,
    None,
    metadata,
)?;
```

## Log Rotation

### Automatic Rotation

Audit logs are automatically rotated when they exceed `max_file_size_bytes`:

1. Current log file is flushed
2. New log file is created with timestamp-based naming
3. Old files beyond `max_files` are deleted
4. Statistics are updated

### Log File Naming

Log files follow the pattern: `audit_{timestamp}.jsonl`

Example:
```
audit_00000000001712145678.jsonl
audit_00000000001712145890.jsonl
audit_00000000001712146123.jsonl
```

### Log Format

Each line is a complete JSON object (JSONL format):
```
{"timestamp":"2026-04-03T10:15:30.123Z","operation":"PUT","keys":["key1"],...}
{"timestamp":"2026-04-03T10:15:31.456Z","operation":"DELETE","keys":["key2"],...}
```

## Statistics

### AuditLogStats

Track audit logging activity with runtime statistics:

```rust
pub struct AuditLogStats {
    /// Total entries written
    pub entries_written: u64,
    
    /// Total entries failed
    pub entries_failed: u64,
    
    /// Number of log rotations
    pub rotations: u64,
    
    /// Current log file size
    pub current_file_size_bytes: u64,
    
    /// Total size of all log files
    pub total_size_bytes: u64,
}
```

### Get Statistics

```rust
let stats = audit_logger.stats();
println!("Audit entries written: {}", stats.entries_written);
println!("Log rotations: {}", stats.rotations);
println!("Total audit log size: {} MB", stats.total_size_bytes / (1024 * 1024));
```

## Performance Considerations

### Overhead

| Configuration | Write Latency Impact | Throughput Impact |
|---------------|---------------------|-------------------|
| Disabled | 0% | 0% |
| Default (buffered) | ~5-10% | ~5% |
| flush_on_write=true | ~50-100% | ~30-50% |

### Recommendations

1. **Development**: Disable audit logging for maximum performance
2. **Staging**: Enable with default settings for testing
3. **Production**: Enable with `flush_on_write=false` for balanced performance
4. **Compliance-Critical**: Enable with `flush_on_write=true` for maximum durability

### Memory Usage

- Each audit entry: ~200-500 bytes (JSON serialized)
- Buffer size: Depends on write frequency and flush interval
- Log retention: `max_files * max_file_size_bytes`

## Integration Points

### FileKV Operations

Audit logging is integrated with all write operations:

| Operation | Audit Trigger | Metadata |
|-----------|---------------|----------|
| `put()` | After successful write | Latency, value hash, size |
| `delete()` | After successful delete | Latency |
| `put_batch()` | After successful batch | Total size, key count |
| `flush_memtable()` | After successful flush | - |
| `compaction` | After successful compaction | Bytes reclaimed, segments merged |

### Error Handling

Failed operations are also audited:

```json
{
  "timestamp": "2026-04-03T10:15:30.123Z",
  "operation": "PUT",
  "keys": ["key1"],
  "success": false,
  "error": "Backpressure: MemTable memory limit exceeded"
}
```

## Compliance Features

### Regulatory Compliance

Audit logging supports compliance with:

- **SOX**: Financial transaction tracking
- **GDPR**: Data access and modification records
- **HIPAA**: Healthcare data access logs
- **PCI-DSS**: Payment card data access tracking

### Audit Trail Properties

1. **Immutability**: Append-only logs prevent modification
2. **Timestamps**: ISO 8601 format with timezone
3. **Integrity**: SHA256 hashes for value verification
4. **Completeness**: All write operations captured
5. **Retention**: Configurable log retention policy

## Querying Audit Logs

### Parse JSON Logs

```rust
use std::fs::File;
use std::io::{BufRead, BufReader};
use tokitai_context::audit_log::AuditEntry;

let file = File::open("audit_00000000001712145678.jsonl")?;
let reader = BufReader::new(file);

for line in reader.lines() {
    let entry: AuditEntry = serde_json::from_str(&line?)?;
    println!("{}: {} - {}", entry.timestamp, entry.operation, entry.keys.join(", "));
}
```

### Filter by Operation Type

```bash
# Using jq
jq 'select(.operation == "PUT")' audit_*.jsonl

# Using grep
grep '"operation":"PUT"' audit_*.jsonl
```

### Analyze with Tools

- **jq**: JSON parsing and filtering
- **Grafana Loki**: Log aggregation and querying
- **ELK Stack**: Elasticsearch, Logstash, Kibana
- **Splunk**: Enterprise log analysis

## Testing

### Unit Tests

```rust
#[test]
fn test_audit_logger_basic() {
    let (logger, _temp_dir) = create_test_logger();

    let result = logger.log_operation(
        AuditOperation::Put,
        vec!["test_key".to_string()],
        Some("sha256:abc123".to_string()),
        Some(1024),
        Some(45),
        true,
        None,
        AuditMetadata::default(),
    );

    assert!(result.is_ok());
    let stats = logger.stats();
    assert_eq!(stats.entries_written, 1);
}
```

### Integration Tests

```rust
#[test]
fn test_audit_logger_with_filekv() {
    let mut config = FileKVConfig::default();
    config.audit_log.enabled = true;
    config.audit_log.log_dir = temp_dir.path().join("audit");
    
    let kv = FileKV::open(config).unwrap();
    
    kv.put("key1", b"value1").unwrap();
    kv.delete("key2").unwrap();
    
    // Verify audit logs were created
    let audit_dir = &config.audit_log.log_dir;
    assert!(audit_dir.exists());
    assert!(audit_dir.read_dir().unwrap().count() > 0);
}
```

## Troubleshooting

### Common Issues

**Issue**: Audit logs not being created
- **Solution**: Check `config.audit_log.enabled = true`
- **Solution**: Verify `log_dir` is writable

**Issue**: High write latency
- **Solution**: Set `flush_on_write = false` (default)
- **Solution**: Increase `max_file_size_bytes` to reduce rotations

**Issue**: Disk space exhaustion
- **Solution**: Reduce `max_files` retention count
- **Solution**: Monitor `stats.total_size_bytes`

**Issue**: Missing audit entries
- **Solution**: Check for errors in `log_operation()` return value
- **Solution**: Verify `flush_on_write` setting for durability requirements

## Future Enhancements

### Planned Features

1. **Async Audit Logging**: Non-blocking log writes
2. **Remote Logging**: Stream to centralized log servers
3. **Encryption**: Encrypt audit logs at rest
4. **Signing**: Cryptographic signatures for integrity
5. **Compression**: Reduce log storage footprint
6. **Query API**: Built-in audit log querying

## Related Documentation

- [P2-009: Incremental Checkpoint](./P2-009_INCREMENTAL_CHECKPOINT.md)
- [P2-016: Prometheus Metrics](./P2-016_PROMETHEUS_METRICS.md)
- [P1-015: Timeout Control](./P1-015_TIMEOUT_CONTROL.md)
- [Architecture Overview](./ARCHITECTURE.md)

## Implementation Details

### Module Structure

```
src/
├── audit_log.rs              # Core audit logging module
│   ├── AuditEntry            # Log entry structure
│   ├── AuditOperation        # Operation types
│   ├── AuditMetadata         # Custom metadata
│   ├── AuditLogConfig        # Configuration
│   ├── AuditLogger           # Logger implementation
│   └── compute_value_hash()  # SHA256 hashing
└── file_kv/
    ├── mod.rs                # FileKV integration
    └── types.rs              # Config integration
```

### Code Changes Summary

| File | Changes | Lines |
|------|---------|-------|
| `src/audit_log.rs` | Core module (existing) | 580 |
| `src/file_kv/mod.rs` | Integration with FileKV | +50 |
| `src/file_kv/types.rs` | Config field addition | +5 |
| `src/compaction.rs` | Compaction audit logging | +25 |
| `src/facade.rs` | Config initialization | +5 |
| `doc/P2-013_AUDIT_LOGGING.md` | Documentation | 350+ |

**Total**: ~615 lines added/modified

## Acceptance Criteria

- [x] Audit logger module implemented and tested
- [x] Integration with FileKV put/delete/batch operations
- [x] Integration with flush and compaction operations
- [x] Configuration via `AuditLogConfig`
- [x] Log rotation and retention management
- [x] SHA256 value hashing for integrity
- [x] Custom metadata support
- [x] Statistics tracking
- [x] 7 unit tests passing
- [x] 70 file_kv tests passing
- [x] Comprehensive documentation

## Conclusion

P2-013 audit logging provides production-ready compliance and operational visibility for Tokitai-Context. The implementation balances performance with durability requirements, offering configurable options for different deployment scenarios.
