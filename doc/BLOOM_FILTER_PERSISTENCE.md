# Bloom Filter Persistence Implementation

## Overview

This document describes the Bloom Filter persistence implementation for the FileKV storage engine in tokitai-context. The implementation enables Bloom Filters to survive restarts, maintaining query performance across sessions.

## Problem Statement

The original Bloom Filter implementation was purely in-memory, meaning:
- Bloom Filters were lost on restart
- Rebuilding required scanning all segment files
- Cold start performance was degraded

## Solution

We implemented persistent Bloom Filters by storing the keys used to construct each filter, enabling:
- Fast save during flush operations
- Quick reload on restart
- Maintained query performance across sessions

## Implementation Details

### File Format

Bloom Filter files use the following format:

```
┌─────────────────────────────────────┐
│ Magic Number (4 bytes)              │ 0x424C4F4F ("BLOO")
├─────────────────────────────────────┤
│ Version (4 bytes)                   │ Currently 1
├─────────────────────────────────────┤
│ Number of Keys (8 bytes)            │ Little-endian u64
├─────────────────────────────────────┤
│ Key 1 Length (4 bytes)              │ Little-endian u32
├─────────────────────────────────────┤
│ Key 1 Data (variable)               │ UTF-8 bytes
├─────────────────────────────────────┤
│ Key 2 Length (4 bytes)              │ ...
├─────────────────────────────────────┤
│ Key 2 Data (variable)               │ ...
├─────────────────────────────────────┤
│ ...                                 │
└─────────────────────────────────────┘
```

### File Naming

Bloom Filter files are named: `bloom_{segment_id:06}.bin`

Example: `bloom_000001.bin`, `bloom_000002.bin`

### Storage Location

Bloom Filter files are stored in the same directory as index files (`index_dir`).

### Key APIs

#### `save_bloom_filter(segment_id, bloom, keys)`

Saves a Bloom Filter to disk during MemTable flush or compaction.

```rust
pub(crate) fn save_bloom_filter(&self, segment_id: u64, _bloom: &bloom::BloomFilter, keys: &[String]) -> Result<()>
```

**Parameters:**
- `segment_id`: The segment ID this Bloom Filter belongs to
- `bloom`: The Bloom Filter instance (currently unused, kept for API consistency)
- `keys`: The list of keys inserted into the Bloom Filter

**Process:**
1. Create file in index directory
2. Write magic number and version
3. Write number of keys
4. Write each key with length prefix
5. Flush to disk

#### `load_bloom_filter(segment_id)`

Loads a Bloom Filter from disk during restart.

```rust
fn load_bloom_filter(&self, segment_id: u64) -> Result<Option<(bloom::BloomFilter, Vec<String>)>>
```

**Returns:**
- `Ok(Some((bloom, keys)))` if file exists and is valid
- `Ok(None)` if file doesn't exist
- `Err(...)` if file is corrupted or invalid

**Process:**
1. Check if file exists
2. Read and validate magic number
3. Read and validate version
4. Read number of keys
5. Read all keys
6. Reconstruct Bloom Filter by inserting all keys
7. Return reconstructed filter

#### `rebuild_bloom_filters()`

Called during `FileKV::open()` to load or rebuild all Bloom Filters.

```rust
pub fn rebuild_bloom_filters(&self) -> Result<usize>
```

**Process:**
1. Iterate all existing segments
2. For each segment:
   - Try to load Bloom Filter from disk
   - If load succeeds: add to memory cache
   - If load fails: rebuild from segment file by scanning all entries
3. Return count of rebuilt filters

**Returns:** Number of Bloom Filters that were rebuilt (not loaded from disk)

### Integration Points

#### MemTable Flush

When MemTable is flushed to a segment:
1. Create new Bloom Filter
2. Insert all keys being flushed
3. Save Bloom Filter to memory map
4. **Save Bloom Filter to disk** (new)
5. Clear MemTable

#### Compaction

When compaction merges segments:
1. Create new Bloom Filter for merged data
2. Insert all live keys (after tombstone removal)
3. Save to memory map
4. **Save to disk** (new)
5. Remove old segment Bloom Filters

#### Restart/Recovery

When FileKV is opened:
1. Load existing segments
2. Load existing indexes
3. **Call `rebuild_bloom_filters()`** (new)
   - Loads from disk if available
   - Rebuilds from segments if not
4. Continue with normal operation

## Performance Characteristics

### Save Operation
- **Time Complexity:** O(K) where K is the number of keys
- **Space Complexity:** O(K * L) where L is average key length
- **I/O:** Single sequential write

### Load Operation
- **Time Complexity:** O(K) to read and reconstruct
- **Space Complexity:** O(K * L) to store keys temporarily
- **I/O:** Single sequential read

### Memory Usage
- In-memory Bloom Filter: ~constant (depends on false positive rate)
- On-disk representation: O(K * L) - stores all keys
- Trade-off: Faster restart vs. larger disk footprint

## Configuration

Bloom Filter behavior is controlled by `FileKVConfig`:

```rust
pub struct FileKVConfig {
    pub enable_bloom: bool,        // Enable/disable Bloom Filters
    // ... other fields
}
```

## Testing

### Test: `test_bloom_filter_persistence`

Verifies:
1. Bloom Filter files are created during flush
2. Data can be read with Bloom Filters enabled
3. Bloom Filter statistics are tracked
4. Bloom Filters are correctly loaded on restart
5. Query performance is maintained after restart

**Test Flow:**
1. Create FileKV instance
2. Write data (triggers flush)
3. Verify Bloom Filter files exist
4. Query existing keys (should succeed)
5. Query non-existing keys (should be filtered)
6. Close and reopen FileKV
7. Verify data still readable
8. Verify Bloom Filters still working

## Limitations

1. **Disk Space:** Storing all keys requires O(K * L) disk space per segment
   - Mitigation: Keys are typically small, and this is a one-time cost per segment

2. **Rebuild Time:** If Bloom Filter file is corrupted, rebuilding from segment requires full scan
   - Mitigation: Rare case, and segment scans are optimized

3. **No Incremental Updates:** Bloom Filter files are written once, never updated
   - This is by design - segments are immutable after creation

## Future Improvements

1. **Bit Vector Serialization:** If the `bloom` crate adds serialization support, we could store the bit vector directly instead of keys
   - Would reduce disk space
   - Would speed up load time

2. **Compression:** Compress key data before writing to disk
   - Could significantly reduce disk usage for large segments

3. **Batch Writes:** Write multiple Bloom Filters in a single I/O operation
   - Could improve flush performance

4. **Background Persistence:** Move Bloom Filter saving to background thread
   - Would reduce flush latency

## Files Modified

- `src/file_kv.rs`: Added Bloom Filter persistence methods
  - `save_bloom_filter()`
  - `load_bloom_filter()`
  - `rebuild_bloom_filters()`
  - Updated `flush_memtable()` to save Bloom Filters
  - Updated `open()` to rebuild Bloom Filters on startup

- `src/compaction.rs`: Updated compaction to save Bloom Filters
  - Modified compaction to collect keys and save Bloom Filter

## Conclusion

The Bloom Filter persistence implementation provides fast restart capabilities while maintaining the query performance benefits of Bloom Filters. The trade-off of increased disk space for faster startup is acceptable for most use cases, especially given the immutable nature of segments in the LSM-Tree architecture.
