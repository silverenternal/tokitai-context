//! Parallel context management modules
//! 
//! Git-style branching for AI agent contexts, supporting:
//! - O(1) fork operations via copy-on-write
//! - 6 merge strategies
//! - Conflict detection and resolution

pub use super::branch::{
    BranchMetadata, BranchState, ConflictType, ContextBranch,
    MergeDecision, MergeStrategy,
};
pub use super::graph::{
    BranchPoint, Conflict, ConflictResolution, ConflictVersion,
    ContextGraph, ContextGraphManager, ContextGraphStats,
    MergeDecision as GraphMergeDecision, MergeRecord, MergedItem,
};
pub use super::merge::{
    compute_diff, BranchDiff, ContextItem as MergeContextItem,
    Merger, MergeResult, ModifiedItem,
};
pub use super::parallel_manager::{
    ParallelContextManager, ParallelContextManagerConfig,
};
pub use super::cow::{
    BranchCloner, CowConfig, CowManager, CowStats, ForkResult,
};
pub use super::cache::{
    AncestorCache, AncestorCacheStats, BranchCache,
    CacheStats as CacheStatsV1, CacheWarmup, CacheWarmupConfig,
    CachedBranch as CachedBranchV1,
};
