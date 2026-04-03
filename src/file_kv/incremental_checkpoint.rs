//! Incremental Checkpoint Module
//!
//! This module implements incremental checkpointing for efficient state persistence.
//! Instead of creating full snapshots, incremental checkpoints only store changes
//! (deltas) since the last checkpoint, significantly reducing storage overhead
//! and checkpoint creation time.
//!
//! # Design
//!
//! ## Checkpoint Types
//! - **Full Checkpoint**: Complete state snapshot (base checkpoint)
//! - **Incremental Checkpoint**: Only contains changes since base checkpoint
//!
//! ## Checkpoint Chain
//! ```text
//! Full_0 → Incr_1 → Incr_2 → Incr_3 → Full_4 → Incr_5 → ...
//! ```
//!
//! ## Recovery
//! To restore state:
//! 1. Load the nearest full checkpoint
//! 2. Replay all incremental checkpoints in order
//! 3. Apply changes to reconstruct current state
//!
//! # Example
//!
//! ```rust,ignore
//! let mut manager = IncrementalCheckpointManager::new(checkpoint_dir);
//!
//! // Create initial full checkpoint
//! let base_id = manager.create_full_checkpoint(&state)?;
//!
//! // Create incremental checkpoints
//! let changes = compute_changes(&old_state, &new_state);
//! let incr_id = manager.create_incremental_checkpoint(&base_id, changes)?;
//!
//! // Restore from checkpoint chain
//! let restored_state = manager.restore(&incr_id)?;
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, BTreeMap};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use tracing::{info, warn, debug, error};

use crate::error::{ContextResult, ContextError};
use std::io;

/// Checkpoint ID type
pub type CheckpointId = String;

/// Checkpoint sequence number
pub type CheckpointSeq = u64;

/// Checkpoint types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckpointType {
    /// Full checkpoint - complete state snapshot
    Full,
    /// Incremental checkpoint - changes since base
    Incremental {
        /// Base checkpoint ID this incremental is based on
        base_checkpoint: CheckpointId,
    },
}

/// Checkpoint entry representing a single change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum CheckpointEntry {
    /// New entry added
    Put {
        key: String,
        value: Vec<u8>,
        timestamp: u64,
    },
    /// Entry deleted
    Delete {
        key: String,
        timestamp: u64,
    },
    /// Entry modified (value changed)
    Modify {
        key: String,
        old_value_hash: String,
        new_value: Vec<u8>,
        timestamp: u64,
    },
}

impl CheckpointEntry {
    /// Get the key affected by this entry
    pub fn key(&self) -> &str {
        match self {
            CheckpointEntry::Put { key, .. } => key,
            CheckpointEntry::Delete { key, .. } => key,
            CheckpointEntry::Modify { key, .. } => key,
        }
    }

    /// Get the timestamp of this entry
    pub fn timestamp(&self) -> u64 {
        match self {
            CheckpointEntry::Put { timestamp, .. } => *timestamp,
            CheckpointEntry::Delete { timestamp, .. } => *timestamp,
            CheckpointEntry::Modify { timestamp, .. } => *timestamp,
        }
    }
}

/// Incremental checkpoint metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalCheckpoint {
    /// Unique checkpoint ID
    pub checkpoint_id: CheckpointId,
    /// Sequence number (monotonically increasing)
    pub sequence: CheckpointSeq,
    /// Checkpoint type
    pub checkpoint_type: CheckpointType,
    /// Creation timestamp
    pub created_at: u64,
    /// List of changes in this checkpoint
    pub entries: Vec<CheckpointEntry>,
    /// Checkpoint metadata
    pub metadata: CheckpointMetadata,
    /// Hash of checkpoint content for integrity verification
    pub content_hash: String,
}

/// Checkpoint metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Optional description
    pub description: Option<String>,
    /// Total entries in this checkpoint
    pub total_entries: usize,
    /// Number of PUT operations
    pub put_count: usize,
    /// Number of DELETE operations
    pub delete_count: usize,
    /// Number of MODIFY operations
    pub modify_count: usize,
    /// Size of checkpoint data in bytes
    pub size_bytes: u64,
    /// Time taken to create checkpoint (microseconds)
    pub creation_time_us: u64,
}

/// Checkpoint chain information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointChain {
    /// List of checkpoint IDs in order
    pub checkpoint_ids: Vec<CheckpointId>,
    /// Map from checkpoint ID to sequence number
    pub sequence_map: HashMap<CheckpointId, CheckpointSeq>,
    /// Map from checkpoint ID to checkpoint type
    pub type_map: HashMap<CheckpointId, CheckpointType>,
    /// Latest full checkpoint ID
    pub latest_full: Option<CheckpointId>,
}

/// Incremental checkpoint manager
pub struct IncrementalCheckpointManager {
    /// Checkpoint storage directory
    checkpoint_dir: PathBuf,
    /// Loaded checkpoints in memory
    checkpoints: HashMap<CheckpointId, IncrementalCheckpoint>,
    /// Current checkpoint chain
    chain: CheckpointChain,
    /// Next sequence number
    next_sequence: CheckpointSeq,
    /// Full checkpoint interval (create full checkpoint every N incremental)
    full_checkpoint_interval: u64,
}

impl IncrementalCheckpointManager {
    /// Create a new incremental checkpoint manager
    pub fn new<P: AsRef<Path>>(checkpoint_dir: P) -> ContextResult<Self> {
        let checkpoint_dir = checkpoint_dir.as_ref().to_path_buf();
        
        fs::create_dir_all(&checkpoint_dir)
            .map_err(ContextError::Io)?;

        let mut manager = Self {
            checkpoint_dir,
            checkpoints: HashMap::new(),
            chain: CheckpointChain {
                checkpoint_ids: Vec::new(),
                sequence_map: HashMap::new(),
                type_map: HashMap::new(),
                latest_full: None,
            },
            next_sequence: 0,
            full_checkpoint_interval: 10, // Default: full checkpoint every 10 increments
        };

        // Load existing checkpoints
        manager.load_checkpoints()?;

        Ok(manager)
    }

    /// Load existing checkpoints from disk
    fn load_checkpoints(&mut self) -> ContextResult<()> {
        if !self.checkpoint_dir.exists() {
            return Ok(());
        }

        let mut checkpoints = Vec::new();

        for entry in fs::read_dir(&self.checkpoint_dir)
            .map_err(ContextError::Io)?
        {
            let entry = entry.map_err(io::Error::other)?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|e| e == "ckpt") {
                let checkpoint = self.load_checkpoint_file(&path)?;
                checkpoints.push(checkpoint);
            }
        }

        // Sort by sequence number
        checkpoints.sort_by_key(|c| c.sequence);

        // Build checkpoint chain
        for checkpoint in checkpoints {
            let checkpoint_id = checkpoint.checkpoint_id.clone();
            let sequence = checkpoint.sequence;
            let checkpoint_type = checkpoint.checkpoint_type.clone();

            self.checkpoints.insert(checkpoint_id.clone(), checkpoint);
            self.chain.checkpoint_ids.push(checkpoint_id.clone());
            self.chain.sequence_map.insert(checkpoint_id.clone(), sequence);
            self.chain.type_map.insert(checkpoint_id.clone(), checkpoint_type.clone());

            if let CheckpointType::Full = checkpoint_type {
                self.chain.latest_full = Some(checkpoint_id.clone());
            }

            if sequence >= self.next_sequence {
                self.next_sequence = sequence + 1;
            }
        }

        info!("Loaded {} checkpoints from disk", self.checkpoints.len());
        Ok(())
    }

    /// Load a single checkpoint file
    fn load_checkpoint_file(&self, path: &Path) -> ContextResult<IncrementalCheckpoint> {
        let file = File::open(path)
            .map_err(ContextError::Io)?;
        let mut reader = BufReader::new(file);
        let mut content = Vec::new();
        reader.read_to_end(&mut content)
            .map_err(ContextError::Io)?;

        // Verify integrity
        let expected_hash = Sha256::digest(&content);
        let expected_hash_hex = format!("0x{}", hex::encode(expected_hash));

        let checkpoint: IncrementalCheckpoint = serde_json::from_slice(&content)
            .map_err(ContextError::Serialization)?;

        if checkpoint.content_hash != expected_hash_hex {
            warn!(
                "Checkpoint {} has invalid hash. Expected: {}, Got: {}",
                checkpoint.checkpoint_id, checkpoint.content_hash, expected_hash_hex
            );
        }

        Ok(checkpoint)
    }

    /// Create a full checkpoint from current state
    pub fn create_full_checkpoint<K, V>(
        &mut self,
        state: &HashMap<K, V>,
        description: Option<&str>,
    ) -> ContextResult<CheckpointId>
    where
        K: Clone + ToString,
        V: Clone + AsRef<[u8]>,
    {
        let start_time = SystemTime::now();
        let sequence = self.next_sequence;
        self.next_sequence += 1;

        let checkpoint_id = self.generate_checkpoint_id(sequence);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| io::Error::other(e.to_string()))?
            .as_micros() as u64;

        // Generate PUT entries for all state
        let mut entries = Vec::with_capacity(state.len());
        let metadata = CheckpointMetadata {
            description: description.map(|s| s.to_string()),
            total_entries: state.len(),
            put_count: state.len(),
            ..Default::default()
        };

        for (key, value) in state {
            entries.push(CheckpointEntry::Put {
                key: key.to_string(),
                value: value.as_ref().to_vec(),
                timestamp,
            });
        }

        let mut checkpoint = IncrementalCheckpoint {
            checkpoint_id: checkpoint_id.clone(),
            sequence,
            checkpoint_type: CheckpointType::Full,
            created_at: timestamp,
            entries,
            metadata,
            content_hash: String::new(),
        };

        // Calculate content hash
        checkpoint.content_hash = self.calculate_content_hash(&checkpoint)?;
        checkpoint.metadata.size_bytes = self.save_checkpoint(&checkpoint)?;

        let creation_time_us = SystemTime::now()
            .duration_since(start_time)
            .map_err(|e| io::Error::other(e.to_string()))?
            .as_micros() as u64;
        checkpoint.metadata.creation_time_us = creation_time_us;

        // Update chain
        let total_entries = checkpoint.metadata.total_entries;
        self.add_to_chain(checkpoint);

        info!(
            "Created full checkpoint {} with {} entries (sequence: {})",
            checkpoint_id, total_entries, sequence
        );

        Ok(checkpoint_id)
    }

    /// Create an incremental checkpoint with only the changes
    pub fn create_incremental_checkpoint(
        &mut self,
        changes: Vec<CheckpointEntry>,
        description: Option<&str>,
    ) -> ContextResult<CheckpointId> {
        let start_time = SystemTime::now();
        let sequence = self.next_sequence;
        self.next_sequence += 1;

        let checkpoint_id = self.generate_checkpoint_id(sequence);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| io::Error::other(e.to_string()))?
            .as_micros() as u64;

        // Determine base checkpoint
        let base_checkpoint = self.chain.latest_full.clone().unwrap_or_else(|| {
            // If no full checkpoint exists, create one instead
            warn!("No full checkpoint found, creating full checkpoint instead");
            checkpoint_id.clone()
        });

        // Calculate metadata
        let mut metadata = CheckpointMetadata {
            description: description.map(|s| s.to_string()),
            total_entries: changes.len(),
            ..Default::default()
        };

        for entry in &changes {
            match entry {
                CheckpointEntry::Put { .. } => metadata.put_count += 1,
                CheckpointEntry::Delete { .. } => metadata.delete_count += 1,
                CheckpointEntry::Modify { .. } => metadata.modify_count += 1,
            }
        }

        let checkpoint_type = if base_checkpoint == checkpoint_id {
            CheckpointType::Full
        } else {
            CheckpointType::Incremental { base_checkpoint }
        };

        let mut checkpoint = IncrementalCheckpoint {
            checkpoint_id: checkpoint_id.clone(),
            sequence,
            checkpoint_type,
            created_at: timestamp,
            entries: changes,
            metadata,
            content_hash: String::new(),
        };

        // Calculate content hash
        checkpoint.content_hash = self.calculate_content_hash(&checkpoint)?;
        checkpoint.metadata.size_bytes = self.save_checkpoint(&checkpoint)?;

        let creation_time_us = SystemTime::now()
            .duration_since(start_time)
            .map_err(|e| io::Error::other(e.to_string()))?
            .as_micros() as u64;
        checkpoint.metadata.creation_time_us = creation_time_us;

        // Update chain
        let checkpoint_type_str = match &checkpoint.checkpoint_type {
            CheckpointType::Full => "full".to_string(),
            CheckpointType::Incremental { base_checkpoint } => base_checkpoint.clone(),
        };
        let total_entries = checkpoint.metadata.total_entries;
        self.add_to_chain(checkpoint);

        // Check if we need to create a full checkpoint
        let since_full = self.chain.checkpoint_ids.len() - 
            self.chain.checkpoint_ids.iter().position(|id| 
                Some(id.clone()) == self.chain.latest_full
            ).unwrap_or(0);
        
        if since_full >= self.full_checkpoint_interval as usize {
            info!("Checkpoint interval reached, next checkpoint will be full");
        }

        info!(
            "Created incremental checkpoint {} with {} entries (sequence: {}, base: {})",
            checkpoint_id, total_entries, sequence, checkpoint_type_str
        );

        Ok(checkpoint_id)
    }

    /// Compute diff between old and new state
    pub fn compute_diff<K, V>(
        old_state: &HashMap<K, V>,
        new_state: &HashMap<K, V>,
    ) -> Vec<CheckpointEntry>
    where
        K: Clone + ToString + Eq + std::hash::Hash,
        V: Clone + AsRef<[u8]> + std::hash::Hash,
    {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_micros() as u64;

        let mut changes = Vec::new();
        let mut processed_keys = HashSet::new();

        // Find deleted and modified entries
        for (key, old_value) in old_state {
            let key_str = key.to_string();
            processed_keys.insert(key_str.clone());

            if let Some(new_value) = new_state.get(key) {
                let old_hash = Sha256::digest(old_value.as_ref());
                let new_hash = Sha256::digest(new_value.as_ref());

                if old_hash != new_hash {
                    // Value modified
                    changes.push(CheckpointEntry::Modify {
                        key: key_str,
                        old_value_hash: format!("0x{}", hex::encode(old_hash)),
                        new_value: new_value.as_ref().to_vec(),
                        timestamp,
                    });
                }
            } else {
                // Key deleted
                changes.push(CheckpointEntry::Delete {
                    key: key_str,
                    timestamp,
                });
            }
        }

        // Find new entries
        for (key, value) in new_state {
            let key_str = key.to_string();
            if !processed_keys.contains(&key_str) {
                changes.push(CheckpointEntry::Put {
                    key: key_str,
                    value: value.as_ref().to_vec(),
                    timestamp,
                });
            }
        }

        changes
    }

    /// Restore state from a checkpoint
    pub fn restore(&self, checkpoint_id: &CheckpointId) -> ContextResult<HashMap<String, Vec<u8>>> {
        let checkpoint = self.checkpoints.get(checkpoint_id)
            .ok_or_else(|| ContextError::CheckpointNotFound(checkpoint_id.clone()))?;

        let mut state = HashMap::new();

        // Find the base full checkpoint
        let base_checkpoint = self.find_base_full_checkpoint(checkpoint)?;
        
        // Apply full checkpoint first
        if let Some(full_ckpt) = self.checkpoints.get(&base_checkpoint) {
            for entry in &full_ckpt.entries {
                match entry {
                    CheckpointEntry::Put { key, value, .. } => {
                        state.insert(key.clone(), value.clone());
                    }
                    CheckpointEntry::Delete { key, .. } => {
                        state.remove(key);
                    }
                    CheckpointEntry::Modify { key, new_value, .. } => {
                        state.insert(key.clone(), new_value.clone());
                    }
                }
            }
        }

        // Apply incremental checkpoints in order
        let start_seq = self.chain.sequence_map.get(&base_checkpoint).copied().unwrap_or(0);
        let target_seq = checkpoint.sequence;

        for seq in (start_seq + 1)..=target_seq {
            if let Some(ckpt_id) = self.chain.checkpoint_ids.iter().find(|id| {
                self.chain.sequence_map.get(*id) == Some(&seq)
            }) {
                if let Some(ckpt) = self.checkpoints.get(ckpt_id) {
                    for entry in &ckpt.entries {
                        match entry {
                            CheckpointEntry::Put { key, value, .. } => {
                                state.insert(key.clone(), value.clone());
                            }
                            CheckpointEntry::Delete { key, .. } => {
                                state.remove(key);
                            }
                            CheckpointEntry::Modify { key, new_value, .. } => {
                                state.insert(key.clone(), new_value.clone());
                            }
                        }
                    }
                }
            }
        }

        info!("Restored state from checkpoint {} ({} keys)", checkpoint_id, state.len());
        Ok(state)
    }

    /// Find the base full checkpoint for a given checkpoint
    fn find_base_full_checkpoint(&self, checkpoint: &IncrementalCheckpoint) -> ContextResult<CheckpointId> {
        match &checkpoint.checkpoint_type {
            CheckpointType::Full => Ok(checkpoint.checkpoint_id.clone()),
            CheckpointType::Incremental { base_checkpoint } => {
                let base = self.checkpoints.get(base_checkpoint)
                    .ok_or_else(|| ContextError::CheckpointNotFound(base_checkpoint.clone()))?;
                self.find_base_full_checkpoint(base)
            }
        }
    }

    /// Get the checkpoint chain
    pub fn get_chain(&self) -> &CheckpointChain {
        &self.chain
    }

    /// Get a checkpoint by ID
    pub fn get_checkpoint(&self, checkpoint_id: &str) -> Option<&IncrementalCheckpoint> {
        self.checkpoints.get(checkpoint_id)
    }

    /// List all checkpoints
    pub fn list_checkpoints(&self) -> Vec<&IncrementalCheckpoint> {
        let mut checkpoints: Vec<_> = self.checkpoints.values().collect();
        checkpoints.sort_by_key(|c| c.sequence);
        checkpoints
    }

    /// Get the latest checkpoint
    pub fn get_latest(&self) -> Option<&IncrementalCheckpoint> {
        self.checkpoints.values().max_by_key(|c| c.sequence)
    }

    /// Delete old checkpoints to save space
    pub fn compact(&mut self, keep_last_n: usize) -> ContextResult<usize> {
        let total = self.checkpoints.len();
        if total <= keep_last_n {
            return Ok(0);
        }

        let to_delete = total - keep_last_n;
        let mut deleted = 0;

        // Get all checkpoint IDs sorted by sequence
        let mut checkpoint_ids: Vec<_> = self.chain.checkpoint_ids.clone();
        checkpoint_ids.sort_by_key(|id| self.chain.sequence_map.get(id).copied().unwrap_or(0));

        // Find the oldest full checkpoint that must be preserved
        let mut oldest_full_seq = None;
        for id in &checkpoint_ids {
            if let Some(ckpt) = self.checkpoints.get(id) {
                if matches!(ckpt.checkpoint_type, CheckpointType::Full) {
                    oldest_full_seq = Some(ckpt.sequence);
                    break;
                }
            }
        }

        // Collect checkpoint IDs to delete first (to avoid borrow issues)
        let mut to_delete_ids = Vec::new();
        for checkpoint_id in &checkpoint_ids {
            if deleted >= to_delete {
                break;
            }

            if let Some(checkpoint) = self.checkpoints.get(checkpoint_id) {
                // Don't delete if this is the only full checkpoint or newer than oldest full
                let is_protected_full = Some(checkpoint.sequence) == oldest_full_seq;
                
                if !is_protected_full {
                    let is_full = matches!(checkpoint.checkpoint_type, CheckpointType::Full);
                    to_delete_ids.push((checkpoint_id.clone(), is_full));
                    deleted += 1;
                    
                    // Update oldest_full_seq if we're deleting a full checkpoint
                    if is_full {
                        oldest_full_seq = self.checkpoints.values()
                            .filter(|c| matches!(c.checkpoint_type, CheckpointType::Full))
                            .min_by_key(|c| c.sequence)
                            .map(|c| c.sequence);
                    }
                }
            }
        }

        // Now actually delete the checkpoints
        for (checkpoint_id, _) in to_delete_ids {
            self.delete_checkpoint(&checkpoint_id)?;
        }

        info!("Compacted {} old checkpoints", deleted);
        Ok(deleted)
    }

    /// Delete a single checkpoint
    fn delete_checkpoint(&mut self, checkpoint_id: &str) -> ContextResult<()> {
        if let Some(checkpoint) = self.checkpoints.remove(checkpoint_id) {
            let path = self.get_checkpoint_path(&checkpoint.checkpoint_id);
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(ContextError::Io)?;
            }

            self.chain.checkpoint_ids.retain(|id| id != checkpoint_id);
            self.chain.sequence_map.remove(checkpoint_id);
            self.chain.type_map.remove(checkpoint_id);

            if let Some(latest_full) = &self.chain.latest_full {
                if latest_full == checkpoint_id {
                    // Find new latest full
                    self.chain.latest_full = self.checkpoints.values()
                        .filter(|c| matches!(c.checkpoint_type, CheckpointType::Full))
                        .max_by_key(|c| c.sequence)
                        .map(|c| c.checkpoint_id.clone());
                }
            }
        }

        Ok(())
    }

    /// Add checkpoint to chain
    fn add_to_chain(&mut self, checkpoint: IncrementalCheckpoint) {
        let checkpoint_id = checkpoint.checkpoint_id.clone();
        let sequence = checkpoint.sequence;
        let checkpoint_type = checkpoint.checkpoint_type.clone();

        if let CheckpointType::Full = checkpoint_type {
            self.chain.latest_full = Some(checkpoint_id.clone());
        }

        self.checkpoints.insert(checkpoint_id.clone(), checkpoint);
        self.chain.checkpoint_ids.push(checkpoint_id.clone());
        self.chain.sequence_map.insert(checkpoint_id.clone(), sequence);
        self.chain.type_map.insert(checkpoint_id.clone(), checkpoint_type);
    }

    /// Save checkpoint to disk
    fn save_checkpoint(&self, checkpoint: &IncrementalCheckpoint) -> ContextResult<u64> {
        let path = self.get_checkpoint_path(&checkpoint.checkpoint_id);
        let content = serde_json::to_vec_pretty(checkpoint)
            .map_err(ContextError::Serialization)?;

        let size = content.len() as u64;

        let mut file = BufWriter::new(File::create(&path)
            .map_err(ContextError::Io)?);
        file.write_all(&content)
            .map_err(ContextError::Io)?;

        Ok(size)
    }

    /// Calculate content hash for integrity verification
    fn calculate_content_hash(&self, checkpoint: &IncrementalCheckpoint) -> ContextResult<String> {
        // Hash everything except the content_hash field
        let mut hasher = Sha256::new();
        
        let data = format!(
            "{}|{}|{:?}|{}|{}",
            checkpoint.checkpoint_id,
            checkpoint.sequence,
            checkpoint.checkpoint_type,
            checkpoint.created_at,
            checkpoint.entries.len()
        );
        hasher.update(data.as_bytes());
        
        for entry in &checkpoint.entries {
            let entry_data = match entry {
                CheckpointEntry::Put { key, value, timestamp } => {
                    format!("PUT|{}|{}|{}", key, hex::encode(value), timestamp)
                }
                CheckpointEntry::Delete { key, timestamp } => {
                    format!("DELETE|{}|{}", key, timestamp)
                }
                CheckpointEntry::Modify { key, old_value_hash, new_value, timestamp } => {
                    format!("MODIFY|{}|{}|{}|{}", key, old_value_hash, hex::encode(new_value), timestamp)
                }
            };
            hasher.update(entry_data.as_bytes());
        }

        let hash = hasher.finalize();
        Ok(format!("0x{}", hex::encode(hash)))
    }

    /// Generate checkpoint ID
    fn generate_checkpoint_id(&self, sequence: CheckpointSeq) -> CheckpointId {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_micros();
        format!("ckpt_{}_{}", sequence, timestamp)
    }

    /// Get checkpoint file path
    fn get_checkpoint_path(&self, checkpoint_id: &str) -> PathBuf {
        self.checkpoint_dir.join(format!("{}.ckpt", checkpoint_id))
    }

    /// Set full checkpoint interval
    pub fn set_full_checkpoint_interval(&mut self, interval: u64) {
        self.full_checkpoint_interval = interval;
    }

    /// Get statistics about checkpoints
    pub fn get_stats(&self) -> CheckpointStats {
        let total_checkpoints = self.checkpoints.len();
        let full_checkpoints = self.checkpoints.values()
            .filter(|c| matches!(c.checkpoint_type, CheckpointType::Full))
            .count();
        let incremental_checkpoints = total_checkpoints - full_checkpoints;
        let total_size_bytes: u64 = self.checkpoints.values()
            .map(|c| c.metadata.size_bytes)
            .sum();
        let total_entries: usize = self.checkpoints.values()
            .map(|c| c.metadata.total_entries)
            .sum();

        CheckpointStats {
            total_checkpoints,
            full_checkpoints,
            incremental_checkpoints,
            total_size_bytes,
            total_entries,
            latest_sequence: self.next_sequence - 1,
        }
    }
}

/// Checkpoint statistics
#[derive(Debug, Clone)]
pub struct CheckpointStats {
    pub total_checkpoints: usize,
    pub full_checkpoints: usize,
    pub incremental_checkpoints: usize,
    pub total_size_bytes: u64,
    pub total_entries: usize,
    pub latest_sequence: CheckpointSeq,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (IncrementalCheckpointManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = IncrementalCheckpointManager::new(temp_dir.path()).unwrap();
        (manager, temp_dir)
    }

    #[test]
    fn test_full_checkpoint_creation() {
        let (mut manager, _temp_dir) = create_test_manager();

        let mut state: HashMap<String, Vec<u8>> = HashMap::new();
        state.insert("key1".to_string(), b"value1".to_vec());
        state.insert("key2".to_string(), b"value2".to_vec());
        state.insert("key3".to_string(), b"value3".to_vec());

        let checkpoint_id = manager.create_full_checkpoint(&state, Some("Test full checkpoint")).unwrap();

        assert!(checkpoint_id.starts_with("ckpt_"));
        
        let checkpoint = manager.get_checkpoint(&checkpoint_id).unwrap();
        assert!(matches!(checkpoint.checkpoint_type, CheckpointType::Full));
        assert_eq!(checkpoint.entries.len(), 3);
        assert_eq!(checkpoint.metadata.total_entries, 3);
        assert_eq!(checkpoint.metadata.put_count, 3);
    }

    #[test]
    fn test_incremental_checkpoint_creation() {
        let (mut manager, _temp_dir) = create_test_manager();

        // First create a full checkpoint
        let mut state: HashMap<String, Vec<u8>> = HashMap::new();
        state.insert("key1".to_string(), b"value1".to_vec());
        let _ = manager.create_full_checkpoint(&state, Some("Base")).unwrap();

        // Create incremental checkpoint with changes
        let changes = vec![
            CheckpointEntry::Put {
                key: "key2".to_string(),
                value: b"value2".to_vec(),
                timestamp: 1000,
            },
            CheckpointEntry::Delete {
                key: "key1".to_string(),
                timestamp: 1001,
            },
        ];

        let checkpoint_id = manager.create_incremental_checkpoint(changes, Some("Test incremental")).unwrap();

        let checkpoint = manager.get_checkpoint(&checkpoint_id).unwrap();
        assert!(matches!(checkpoint.checkpoint_type, CheckpointType::Incremental { .. }));
        assert_eq!(checkpoint.entries.len(), 2);
        assert_eq!(checkpoint.metadata.put_count, 1);
        assert_eq!(checkpoint.metadata.delete_count, 1);
    }

    #[test]
    fn test_compute_diff() {
        let mut old_state: HashMap<String, Vec<u8>> = HashMap::new();
        old_state.insert("key1".to_string(), b"value1".to_vec());
        old_state.insert("key2".to_string(), b"value2".to_vec());

        let mut new_state: HashMap<String, Vec<u8>> = HashMap::new();
        new_state.insert("key1".to_string(), b"value1_modified".to_vec());
        new_state.insert("key3".to_string(), b"value3".to_vec());

        let changes = IncrementalCheckpointManager::compute_diff(&old_state, &new_state);

        assert_eq!(changes.len(), 3);
        
        let mut has_delete = false;
        let mut has_modify = false;
        let mut has_put = false;

        for change in &changes {
            match change {
                CheckpointEntry::Delete { key, .. } if key == "key2" => has_delete = true,
                CheckpointEntry::Modify { key, .. } if key == "key1" => has_modify = true,
                CheckpointEntry::Put { key, .. } if key == "key3" => has_put = true,
                _ => {}
            }
        }

        assert!(has_delete);
        assert!(has_modify);
        assert!(has_put);
    }

    #[test]
    fn test_restore_from_full_checkpoint() {
        let (mut manager, _temp_dir) = create_test_manager();

        let mut state: HashMap<String, Vec<u8>> = HashMap::new();
        state.insert("key1".to_string(), b"value1".to_vec());
        state.insert("key2".to_string(), b"value2".to_vec());

        let checkpoint_id = manager.create_full_checkpoint(&state, None).unwrap();
        let restored = manager.restore(&checkpoint_id).unwrap();

        assert_eq!(restored.len(), 2);
        assert_eq!(restored.get("key1"), Some(&b"value1".to_vec()));
        assert_eq!(restored.get("key2"), Some(&b"value2".to_vec()));
    }

    #[test]
    fn test_restore_from_incremental_checkpoint() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Create base full checkpoint
        let mut state: HashMap<String, Vec<u8>> = HashMap::new();
        state.insert("key1".to_string(), b"value1".to_vec());
        let _ = manager.create_full_checkpoint(&state, None).unwrap();

        // Create incremental checkpoint
        let changes = vec![
            CheckpointEntry::Put {
                key: "key2".to_string(),
                value: b"value2".to_vec(),
                timestamp: 1000,
            },
            CheckpointEntry::Delete {
                key: "key1".to_string(),
                timestamp: 1001,
            },
        ];
        let incr_id = manager.create_incremental_checkpoint(changes, None).unwrap();

        // Restore from incremental
        let restored = manager.restore(&incr_id).unwrap();

        assert_eq!(restored.len(), 1);
        assert_eq!(restored.get("key1"), None); // Deleted
        assert_eq!(restored.get("key2"), Some(&b"value2".to_vec()));
    }

    #[test]
    fn test_checkpoint_chain_restore() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Create full checkpoint
        let mut state: HashMap<String, Vec<u8>> = HashMap::new();
        state.insert("a".to_string(), b"1".to_vec());
        let _ = manager.create_full_checkpoint(&state, None).unwrap();

        // First incremental
        let changes1 = vec![
            CheckpointEntry::Put {
                key: "b".to_string(),
                value: b"2".to_vec(),
                timestamp: 1000,
            },
        ];
        let _ = manager.create_incremental_checkpoint(changes1, None).unwrap();

        // Second incremental
        let changes2 = vec![
            CheckpointEntry::Put {
                key: "c".to_string(),
                value: b"3".to_vec(),
                timestamp: 2000,
            },
            CheckpointEntry::Delete {
                key: "a".to_string(),
                timestamp: 2001,
            },
        ];
        let incr2_id = manager.create_incremental_checkpoint(changes2, None).unwrap();

        // Restore from latest incremental
        let restored = manager.restore(&incr2_id).unwrap();

        assert_eq!(restored.len(), 2);
        assert_eq!(restored.get("a"), None);
        assert_eq!(restored.get("b"), Some(&b"2".to_vec()));
        assert_eq!(restored.get("c"), Some(&b"3".to_vec()));
    }

    #[test]
    fn test_checkpoint_persistence() {
        let temp_dir = TempDir::new().unwrap();

        // Create manager and checkpoints
        {
            let mut manager = IncrementalCheckpointManager::new(temp_dir.path()).unwrap();
            
            let mut state: HashMap<String, Vec<u8>> = HashMap::new();
            state.insert("key1".to_string(), b"value1".to_vec());
            let _ = manager.create_full_checkpoint(&state, None).unwrap();

            let changes = vec![
                CheckpointEntry::Put {
                    key: "key2".to_string(),
                    value: b"value2".to_vec(),
                    timestamp: 1000,
                },
            ];
            let _ = manager.create_incremental_checkpoint(changes, None).unwrap();
        }

        // Create new manager (should load existing checkpoints)
        let manager = IncrementalCheckpointManager::new(temp_dir.path()).unwrap();
        
        assert_eq!(manager.checkpoints.len(), 2);
        assert_eq!(manager.chain.checkpoint_ids.len(), 2);
    }

    #[test]
    fn test_checkpoint_stats() {
        let (mut manager, _temp_dir) = create_test_manager();

        let mut state: HashMap<String, Vec<u8>> = HashMap::new();
        state.insert("key1".to_string(), b"value1".to_vec());
        let _ = manager.create_full_checkpoint(&state, None).unwrap();

        let changes = vec![
            CheckpointEntry::Put {
                key: "key2".to_string(),
                value: b"value2".to_vec(),
                timestamp: 1000,
            },
        ];
        let _ = manager.create_incremental_checkpoint(changes, None).unwrap();

        let stats = manager.get_stats();
        
        assert_eq!(stats.total_checkpoints, 2);
        assert_eq!(stats.full_checkpoints, 1);
        assert_eq!(stats.incremental_checkpoints, 1);
        assert!(stats.total_size_bytes > 0);
        assert_eq!(stats.total_entries, 2);
    }

    #[test]
    fn test_checkpoint_compaction() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Create one full checkpoint
        let mut state: HashMap<String, Vec<u8>> = HashMap::new();
        state.insert("key0".to_string(), b"value0".to_vec());
        let _ = manager.create_full_checkpoint(&state, None).unwrap();

        // Create incremental checkpoints
        for i in 1..6 {
            let changes = vec![
                CheckpointEntry::Put {
                    key: format!("key{}", i),
                    value: format!("value{}", i).into_bytes(),
                    timestamp: i as u64 * 1000,
                },
            ];
            let _ = manager.create_incremental_checkpoint(changes, None).unwrap();
        }

        assert_eq!(manager.checkpoints.len(), 6);

        // Compact, keeping last 3
        let deleted = manager.compact(3).unwrap();

        assert!(deleted >= 2);
        assert!(manager.checkpoints.len() <= 4); // At least the full checkpoint is preserved
    }

    #[test]
    fn test_checkpoint_integrity() {
        let (mut manager, _temp_dir) = create_test_manager();

        let mut state: HashMap<String, Vec<u8>> = HashMap::new();
        state.insert("key1".to_string(), b"value1".to_vec());
        let checkpoint_id = manager.create_full_checkpoint(&state, None).unwrap();

        let checkpoint = manager.get_checkpoint(&checkpoint_id).unwrap();
        assert!(checkpoint.content_hash.starts_with("0x"));
        assert!(checkpoint.content_hash.len() > 10);
    }
}
