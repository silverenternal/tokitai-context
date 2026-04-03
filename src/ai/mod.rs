//! AI integration modules (optional feature)
//! 
//! AI-powered features for conflict resolution, purpose inference,
//! smart merge recommendations, and branch summarization.
//! 
//! Requires the `ai` feature to be enabled.

#[cfg(feature = "ai")]
pub use super::ai_resolver::{
    AIConflictResolver, ConflictResolutionRequest, ConflictResolutionResponse,
    ConflictAnalysisReport, ResolverStats,
};
#[cfg(feature = "ai")]
pub use super::purpose_inference::{
    AIPurposeInference, PurposeInferenceRequest, PurposeInferenceResult,
    BranchType, InferenceStats,
};
#[cfg(feature = "ai")]
pub use super::smart_merge::{
    AISmartMergeRecommender, MergeRecommendationRequest, MergeRecommendation,
    TimingRecommendation, RiskAssessment, ChecklistItem, ChecklistStatus,
    QuickAssessment, RecommenderStats,
};
#[cfg(feature = "ai")]
pub use super::summarizer::{
    AIBranchSummarizer, SummaryGenerationRequest, SummaryGenerationResult,
    TimelineEvent, StatusAssessment, MergeReadiness, QuickSummary, SummarizerStats,
};
#[cfg(feature = "ai")]
pub use super::ai_enhanced_manager::{
    AIEnhancedContextManager, AIStats,
};

// Semantic index is available without AI feature for basic search
pub use super::semantic_index::{
    SemanticIndex, SemanticIndexConfig, SemanticIndexManager,
    FingerprintIndexEntry as SearchIndexEntry, IndexStats, SearchResult,
};
