# Data Consistency Check Tool

## Overview

The Data Consistency Check Tool provides mechanisms to verify data integrity and consistency across different storage backends in the tokitai-context system. It helps detect synchronization issues, corruption, and configuration problems.

## Motivation

The tokitai-context system uses multiple storage backends:
- **FileKV**: LSM-tree based storage with MemTable, Segments, and BlockCache
- **FileService**: Traditional file-based storage with hash indexing

When both backends are enabled (via `enable_filekv_backend`), data should be synchronized between them. The consistency checker helps ensure:
1. Data exists in both backends when expected
2. Content hashes match between backends
3. Index references are valid
4. Bloom filters are accurate

## Usage

### Basic Check

```rust
use tokitai_context::consistency_check::{ConsistencyChecker, CheckReport};

fn main() -> anyhow::Result<()> {
    // Create checker for context root directory
    let checker = ConsistencyChecker::new("./.context")?;
    
    // Run full consistency check
    let report = checker.run_full_check()?;
    
    // Review results
    println!("FileKV entries: {}", report.filekv_entries);
    println!("FileService entries: {}", report.file_service_entries);
    println!("Inconsistencies: {}", report.inconsistency_count);
    
    if !report.is_consistent {
        for issue in &report.inconsistencies {
            eprintln!("Issue: {:?}", issue);
        }
    }
    
    Ok(())
}
```

### Custom Configuration

```rust
use tokitai_context::consistency_check::{
    ConsistencyChecker, ConsistencyCheckerConfig,
};

fn main() -> anyhow::Result<()> {
    let config = ConsistencyCheckerConfig {
        filekv_only: true,        // Only check FileKV (faster)
        check_bloom_filters: false, // Skip bloom filter checks
        check_index_integrity: false, // Skip index checks
        fail_fast: true,          // Stop on first error
    };
    
    let checker = ConsistencyChecker::with_config("./.context", config)?;
    let report = checker.run_full_check()?;
    
    Ok(())
}
```

### Repair Operations

```rust
use tokitai_context::consistency_check::ConsistencyChecker;

fn main() -> anyhow::Result<()> {
    let checker = ConsistencyChecker::new("./.context")?;
    
    // Run check
    let report = checker.run_full_check()?;
    
    if !report.is_consistent {
        // Attempt automatic repair
        let repair_report = checker.repair(&report)?;
        
        println!("Repaired {} keys", repair_report.repaired.len());
        println!("Unfixable: {}", repair_report.unfixable.len());
        
        for unfixable in &repair_report.unfixable {
            eprintln!("Manual fix needed: {} - {}", unfixable.key, unfixable.reason);
        }
    }
    
    Ok(())
}
```

## Issue Types

The checker detects several types of consistency issues:

### KeyOnlyInFileKV

Key exists in FileKV but not in FileService.

```rust
ConsistencyIssue::KeyOnlyInFileKV { key: String }
```

**Possible Causes:**
- Sync failure during write operation
- FileService backend disabled or misconfigured
- Partial write during crash

**Resolution:** Sync key to FileService

### KeyOnlyInFileService

Key exists in FileService but not in FileKV.

```rust
ConsistencyIssue::KeyOnlyInFileService { key: String }
```

**Possible Causes:**
- FileKV backend recently enabled
- Sync failure during write operation
- Index corruption in FileKV

**Resolution:** Sync key to FileKV

### ContentMismatch

Same key exists in both backends but content differs.

```rust
ConsistencyIssue::ContentMismatch {
    key: String,
    filekv_hash: String,
    file_service_hash: String,
}
```

**Possible Causes:**
- Concurrent writes without proper locking
- Corruption in one backend
- Version skew during update

**Resolution:** Determine authoritative source, resync other backend

### CorruptedData

Data corruption detected in one backend.

```rust
ConsistencyIssue::CorruptedData {
    key: String,
    backend: String,
    error: String,
}
```

**Possible Causes:**
- Disk corruption
- Incomplete write during crash
- Checksum mismatch

**Resolution:** Restore from backup or other backend

### InvalidIndexReference

Index points to non-existent segment or offset.

```rust
ConsistencyIssue::InvalidIndexReference {
    key: String,
    segment_id: u64,
    offset: u64,
}
```

**Possible Causes:**
- Segment file deleted
- Index not updated after compaction
- Corruption in index file

**Resolution:** Rebuild index from segments

### BloomFilterMismatch

Bloom filter gives incorrect result.

```rust
ConsistencyIssue::BloomFilterMismatch {
    key: String,
    bloom_filter_says_exists: bool,
    actual_exists: bool,
}
```

**Possible Causes:**
- Bloom filter not updated after write
- Filter corruption
- Filter not rebuilt after compaction

**Resolution:** Rebuild bloom filter

## Check Report

The `CheckReport` structure contains:

```rust
pub struct CheckReport {
    /// Total entries in FileKV
    pub filekv_entries: usize,
    /// Total entries in FileService
    pub file_service_entries: usize,
    /// Number of inconsistencies found
    pub inconsistency_count: usize,
    /// Detailed list of issues
    pub inconsistencies: Vec<ConsistencyIssue>,
    /// Check duration in milliseconds
    pub duration_ms: u64,
    /// Overall consistency status
    pub is_consistent: bool,
}
```

## Repair Report

The `RepairReport` structure contains:

```rust
pub struct RepairReport {
    /// Successfully repaired keys
    pub repaired: Vec<RepairEntry>,
    /// Keys that couldn't be repaired
    pub unfixable: Vec<UnfixableEntry>,
    /// Repair duration in milliseconds
    pub duration_ms: u64,
}
```

## Integration with CI/CD

Use the consistency checker in CI/CD pipelines to catch data integrity issues early:

```rust
#[cfg(test)]
mod integration_tests {
    use tokitai_context::consistency_check::{
        ConsistencyChecker, ConsistencyCheckerConfig,
    };

    #[test]
    fn test_data_consistency() -> anyhow::Result<()> {
        let config = ConsistencyCheckerConfig {
            fail_fast: true,
            ..Default::default()
        };
        
        let checker = ConsistencyChecker::with_config("./.context", config)?;
        let report = checker.run_full_check()?;
        
        assert!(report.is_consistent, "Data consistency check failed: {:?}", report.inconsistencies);
        
        Ok(())
    }
}
```

## Performance Considerations

- **Full Check**: Scans all entries in both backends, O(n) where n = total entries
- **FileKV Only**: ~10x faster, suitable for quick checks
- **Fail Fast**: Stops on first error, useful for CI/CD
- **Bloom Filter Check**: Additional I/O, may be slow for large datasets

### Recommended Configurations

**Development:**
```rust
ConsistencyCheckerConfig {
    filekv_only: false,
    check_bloom_filters: true,
    check_index_integrity: true,
    fail_fast: false,
}
```

**Production (Periodic):**
```rust
ConsistencyCheckerConfig {
    filekv_only: false,
    check_bloom_filters: true,
    check_index_integrity: true,
    fail_fast: false,
}
```

**CI/CD (Fast):**
```rust
ConsistencyCheckerConfig {
    filekv_only: true,
    check_bloom_filters: false,
    check_index_integrity: false,
    fail_fast: true,
}
```

## Limitations

1. **No Iterator API**: Current FileKV and FileService don't expose iteration APIs, limiting the checker's ability to enumerate all entries. This is planned for future enhancement.

2. **Read-Only**: The checker doesn't modify data. Repair operations require manual intervention.

3. **No Live Checking**: The checker operates on disk state, not in-flight operations.

## Future Enhancements

- [ ] Add iterator API to FileKV and FileService
- [ ] Implement live consistency checking (during operations)
- [ ] Add automatic background consistency monitoring
- [ ] Integrate with metrics export (Prometheus)
- [ ] Support incremental checks (only check new/modified entries)
- [ ] Add repair automation for common issues

## Troubleshooting

### Checker Reports Many KeyOnlyInFileKV Issues

This is normal if FileKV backend was recently enabled. Run repair to sync to FileService, or disable FileKV if not needed.

### Check Takes Too Long

Enable `filekv_only` mode or `fail_fast` for faster checks. Consider running checks during off-peak hours.

### Repair Fails for Some Keys

Check logs for specific error messages. Some issues (e.g., corruption in both backends) require manual intervention or backup restoration.
