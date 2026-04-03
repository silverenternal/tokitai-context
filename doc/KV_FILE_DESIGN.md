# 纯文件 KV 性能优化方案

## 核心思想

**不引入数据库依赖，通过文件组织优化达到接近 KV 存储的性能。**

参考 RocksDB/LevelDB 的 LSM-Tree 思想，但用纯文件实现。

---

## 当前性能瓶颈分析

### 当前架构（v0.2.0）
```
写入流程：
1. 计算 SHA256 哈希 (39µs 中的 ~5µs)
2. 写入文件内容 (39µs 中的 ~25µs) ← 瓶颈
3. 创建符号链接 (39µs 中的 ~9µs)
4. 追加日志 (可选，~5µs)

读取流程：
1. 读取符号链接获取路径 (~2µs)
2. 打开文件 + 读取内容 (~42µs) ← 瓶颈
```

**瓶颈根源**：
- 每个 key 一个文件 → 大量小文件 I/O
- 没有批量写入机制 → 频繁 fsync
- 没有内存缓存 → 每次读取都访问磁盘

---

## 优化方案：文件式 LSM-Tree

### 架构设计

```
.context/
├── memtable/              # 内存表（内存中）
│   └── pending_writes     # 待刷盘的写入缓冲
│
├── segments/              # SSTable 式数据段
│   ├── segment_0001.log   # 顺序写入的数据段
│   ├── segment_0002.log
│   └── segment_0003.log
│
├── index/                 # 稀疏索引
│   ├── index_0001.bin     # 段内偏移索引
│   ├── index_0002.bin
│   └── index_0003.bin
│
├── manifest.bin           # 元数据（当前段、索引映射）
└── wal.log                # 预写日志（崩溃恢复）
```

### 核心数据结构

#### 1. MemTable（内存缓冲）
```rust
/// 内存写入缓冲（类似 RocksDB MemTable）
struct MemTable {
    /// 跳表：key → (value_offset, value_len)
    data: BTreeMap<String, ValuePointer>,
    /// 当前大小（字节）
    size_bytes: usize,
    /// 刷盘阈值
    flush_threshold: usize, // 默认 4MB
}

/// 值指针（指向 segment 文件）
struct ValuePointer {
    segment_id: u64,    // 段文件 ID
    offset: u64,        // 段内偏移
    len: u32,           // 值长度
    checksum: u32,      // CRC32 校验和
}
```

#### 2. Segment 文件（顺序写入）
```
Segment 文件格式：
┌─────────────────────────────────────┐
│ Entry 1                             │
│ ├─ Key Length (u32)                 │
│ ├─ Key Bytes                        │
│ ├─ Value Length (u32)               │
│ ├─ Value Bytes                      │
│ ├─ Checksum (CRC32)                 │
├─────────────────────────────────────┤
│ Entry 2                             │
│ ...                                 │
└─────────────────────────────────────┘
```

#### 3. 稀疏索引（Index Block）
```rust
/// 段索引（每 N 条记录一个索引点）
struct SegmentIndex {
    /// 索引点：key → (offset, seq_num)
    /// 每隔 100 条记录建立一个索引点
    index_points: BTreeMap<String, u64>,
    /// 段文件路径
    segment_path: PathBuf,
}

// 索引文件格式（二进制）：
// ┌──────────────────┐
// │ Magic: "TCIX"    │ 4 bytes
// │ Version: u32     │ 4 bytes
// │ Entry Count      │ 4 bytes
// ├──────────────────┤
// │ Index Entry 1    │
// │ ├─ Key Len       │
// │ ├─ Key           │
// │ ├─ Offset (u64)  │
// ├──────────────────┤
// │ ...              │
// └──────────────────┘
```

---

## 性能优化技术

### 1. 批量写入（Batch Write）

**当前**：每次写入都 fsync
```rust
// 39µs/次
file.write_all(content)?;
file.sync_all()?;  // ← 慢
```

**优化后**：批量刷盘
```rust
// 写入 MemTable（内存操作，~1µs）
memtable.insert(key, value);

// 达到阈值后批量刷盘（4MB 一次）
if memtable.size() > 4MB {
    flush_memtable_to_segment()?;  // 摊还 ~5µs/次
}
```

**预期提升**：写入 **7-10x**（39µs → 5µs）

---

### 2. 顺序写入（Sequential Write）

**当前**：随机文件写入
```
写入 A → 文件 A（随机位置 1）
写入 B → 文件 B（随机位置 2）
写入 C → 文件 C（随机位置 3）
```

**优化后**：顺序追加到 segment
```
写入 A, B, C → segment_001.log（连续追加）
```

**预期提升**：写入吞吐 **3-5x**

---

### 3. 稀疏索引 + 二分查找

**当前**：符号链接 → 文件路径（需要 stat 系统调用）

**优化后**：内存索引 + 二分查找
```rust
// 索引常驻内存
index: BTreeMap<String, ValuePointer>

// 查找 O(log N)，内存操作 ~1µs
let ptr = index.get(key)?;
let file = &segment_files[ptr.segment_id];
file.seek(SeekFrom::Start(ptr.offset))?;
```

**预期提升**：读取 **20-40x**（44µs → 1-2µs）

---

### 4. 内存缓存层

```rust
/// 热点数据缓存
struct BlockCache {
    /// LRU 缓存：segment_id + offset → data
    cache: LruCache<(u64, u64), Arc<[u8]>>,
    /// 缓存大小（默认 64MB）
    max_size: usize,
}

// 读取流程：
// 1. 检查缓存（命中 ~0.5µs）
// 2. 未命中才读磁盘（~50µs）
```

**预期提升**：热点读取 **50-100x**（缓存命中时）

---

### 5. 布隆过滤器（快速判断 key 不存在）

```rust
/// 每个 segment 一个 Bloom Filter
struct SegmentBloom {
    filter: BloomFilter,
    false_positive_rate: f64, // 默认 1%
}

// 读取前检查：
if !bloom_filter.might_contain(&key) {
    return Ok(None);  // 肯定不存在，无需读磁盘
}
// 可能存在，继续查找
```

**预期提升**：不存在查询 **100x**（避免无效 I/O）

---

### 6. 零拷贝读取（mmap）

```rust
// 当前：read_to_end 需要拷贝
let mut buf = Vec::new();
file.read_to_end(&mut buf)?;  // 内核 → 用户空间拷贝

// 优化：mmap 零拷贝
let mmap = unsafe { Mmap::map(&file)? };
let data = &mmap[offset..offset+len];  // 直接访问
```

**预期提升**：大值读取 **2-3x**

---

## 实现方案

### 阶段 1：基础 LSM 结构（1-2 天）

```rust
// src/file_kv.rs
pub struct FileKV {
    /// 当前 MemTable
    memtable: MemTable,
    /// 已刷盘的 segments
    segments: Vec<SegmentFile>,
    /// 索引：key → segment_id + offset
    index: BTreeMap<String, ValuePointer>,
    /// WAL（崩溃恢复）
    wal: WalManager,
}

impl FileKV {
    pub fn put(&mut self, key: &str, value: &[u8]) -> Result<()>;
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    pub fn delete(&mut self, key: &str) -> Result<()>;
    pub fn iter(&self) -> Result<SegmentIterator>;
}
```

### 阶段 2：索引优化（1 天）

```rust
// src/file_index.rs
pub struct SparseIndex {
    /// 稀疏索引点（每 100 条记录一个）
    index_points: Vec<IndexPoint>,
    /// 段文件 mmap
    mmap: Mmap,
}

impl SparseIndex {
    pub fn find(&self, key: &str) -> Option<u64>;  // 二分查找
}
```

### 阶段 3：缓存层（1 天）

```rust
// src/kv_cache.rs
pub struct KvCache {
    data_cache: LruCache<String, Arc<[u8]>>,
    index_cache: BTreeMap<String, ValuePointer>,
}
```

### 阶段 4：压缩与合并（1-2 天）

```rust
// src/compaction.rs
pub struct Compactor {
    /// 合并小 segment 为大 segment
    /// 清理删除标记
    /// 重建索引
}
```

---

## 预期性能对比

| 操作 | 当前文件 | 优化后文件 | RocksDB | 提升幅度 |
|------|---------|-----------|---------|---------|
| **写入** | 39.66 µs | **5-7 µs** | 5-10 µs | **6-8x** ✅ |
| **读取（冷）** | 44.07 µs | **2-3 µs** | 1-3 µs | **15-22x** ✅ |
| **读取（热）** | 44.07 µs | **0.5-1 µs** | 0.5-1 µs | **44-88x** ✅ |
| **范围查询** | O(N) | **O(log N)** | O(log N) | **100x+** ✅ |
| **删除** | ~40 µs | **~1 µs**（标记） | ~1 µs | **40x** ✅ |
| **批量写入** | N×40µs | **N×5µs** | N×5µs | **8x** ✅ |

---

## 优缺点分析

### ✅ 优点

1. **零依赖**：无需 rocksdb-sys、librocksdb
2. **纯 Rust 实现**：编译快、跨平台
3. **二进制体积小**：增加 ~500KB 代码
4. **性能接近 RocksDB**：90%+ 性能
5. **调试友好**：segment 文件可直接查看
6. **向后兼容**：可与现有文件存储共存

### ⚠️ 缺点

1. **代码复杂度增加**：需要实现 LSM-Tree 逻辑
2. **崩溃恢复复杂**：需要 WAL + checkpoint
3. **压缩/合并需手动实现**：RocksDB 已高度优化
4. **并发控制需自研**：无 MVCC 支持

---

## 与现有架构集成

### 方案 A：完全替换
```rust
// 替换 HashIndex + StorageLayer
pub struct FileKVBackend {
    kv: FileKV,
}

impl FileContextService for FileKVBackend {
    fn add(&mut self, session: &str, content: &[u8], layer: ContentType) -> Result<String> {
        let hash = compute_hash(content);
        self.kv.put(&hash, content)?;
        Ok(hash)
    }
    
    fn get_by_hash(&self, hash: &str) -> Result<Vec<u8>> {
        self.kv.get(hash)?.ok_or_else(|| anyhow!("Not found"))
    }
}
```

### 方案 B：混合架构（推荐）
```
热数据（短期层） → FileKV（高性能）
冷数据（长期层） → 文件系统（易调试）
```

---

## 实施建议

### 优先级
1. **MemTable + Segment**（核心，2 天）
2. **稀疏索引**（关键，1 天）
3. **WAL 崩溃恢复**（可靠性，1 天）
4. **Block Cache**（热点优化，1 天）
5. **Bloom Filter**（可选，0.5 天）
6. **压缩/合并**（可选，2 天）

### Feature Flag
```toml
[features]
default = ["wal"]
kv = []  # 启用 FileKV 后端
```

### 代码量估算
- 核心实现：~1500 行
- 测试：~800 行
- 文档：~200 行
- **总计**：~2500 行

---

## 结论

**纯文件方案可以达到 KV 存储 90%+ 的性能**，核心优化：
1. **MemTable 批量写入** → 写入 6-8x 提升
2. **内存索引 + 稀疏索引** → 读取 15-22x 提升
3. **Block Cache** → 热点读取 44-88x 提升
4. **Bloom Filter** → 无效查询 100x 提升

**无需引入数据库依赖，保持纯文件优势。**

是否开始实施？
