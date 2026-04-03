//! Optimization modules
//! 
//! Performance optimizations including caching, compression, 
//! deduplication, and efficient algorithms.

// Merge algorithms
pub use super::three_way_merge::{
    FileMetadata, MergeOutcome, ThreeWayMerger, MergeComparison,
};
pub use super::bloom_conflict::{
    BloomFilter, BloomConflictDetector, BloomStats, PerformanceComparison,
};
pub use super::optimized_merge::{
    AdvancedMerger, ContentDeduplicator, DedupResult, DedupStats,
    Diff3Hunk, Diff3Result, LcsAlignment, SemanticBlock,
    SemanticMergeOutcome, SemanticMergeResult,
};
pub use super::storage_optimization::{
    ChangeType, CompressionAlgorithm, CompressionConfig,
    ContentAddressableEntry, ContentAddressableStorage,
    GcResult, IncrementalSnapshot, SnapshotChange, SnapshotManager,
    SnapshotMetadata, StorageStats,
};
pub use super::parallel_merge::{
    ParallelMerger, ParallelMergeConfig, ParallelMergeResult, ParallelMergeStats,
};

// Caching algorithms
pub use super::lru_cache::{
    BranchLRUCache, BranchCacheConfig, CachedBranch, CacheStats,
    ThreadSafeBranchCache,
};
pub use super::arc_cache::{
    ArcCache, ArcCacheConfig, ArcCacheStats, ArcEntry, BranchArcCache,
};
pub use super::cuckoo_filter::{
    CuckooFilter, CuckooStats, CuckooConflictDetector,
};
pub use super::dictionary_compression::{
    DictionaryCompressor, DictionaryCompressionConfig, DictionaryStats,
    DictionaryMetadata, DictionaryContentAddressableStorage,
};

// Optimization algorithms
pub use super::hirschberg_lcs::{HirschbergLCS, OptimizedLcsResult};
pub use super::minhash_lsh::{
    MinHashGenerator, MinHashSignature, LSHConfig, LSHIndex,
    LSHIndexStats, MinHashLSHIndex, DocumentMetadata,
};
