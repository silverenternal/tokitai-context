# P3-006: FUSE Filesystem Interface

## Overview

This module provides a FUSE (Filesystem in Userspace) interface for tokitai-context, allowing the key-value store to be mounted and accessed as a regular filesystem.

## Features

- **Mount KV Store as Filesystem**: Access key-value data through standard file operations
- **Column Family as Directories**: Each column family appears as a directory
- **Standard File Operations**: read, write, create, delete, mkdir, rmdir
- **Automatic Attribute Management**: File metadata handled automatically
- **Async-compatible**: Can be integrated with async runtimes

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Userspace Applications                   │
│                     (cat, ls, vim, etc.)                     │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼ FUSE /dev/fuse
┌─────────────────────────────────────────────────────────────┐
│                    FUSE Kernel Module                        │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   tokitai-fuse (Userspace)                   │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                 fuser::Filesystem                     │  │
│  │  - lookup()    - getattr()    - readdir()            │  │
│  │  - read()      - write()      - create()             │  │
│  │  - unlink()    - mkdir()      - rmdir()              │  │
│  └──────────────────────────────────────────────────────┘  │
│                            │                                │
│                            ▼                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              ColumnFamilyManager                      │  │
│  │  - default/    - users/    - sessions/               │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Basic Mount

```rust
use tokitai_context::fuse_fs::{FuseFS, FuseConfig};

fn main() -> anyhow::Result<()> {
    let config = FuseConfig::new("/mnt/tokitai");
    let fs = FuseFS::new(config);
    
    // Mount and run (blocking)
    fs.run()?;
    
    Ok(())
}
```

### With Custom Configuration

```rust
let config = FuseConfig::new("/mnt/tokitai")
    .with_root_path("/var/lib/tokitai")
    .with_debug(true);

let fs = FuseFS::new(config);
fs.run()?;
```

### Background Mount

```rust
use std::thread;

let config = FuseConfig::new("/mnt/tokitai");
let fs = FuseFS::new(config);

// Mount in background thread
thread::spawn(move || {
    fs.run().unwrap();
});

// Main thread continues...
```

## Usage

### Mounting

```bash
# Create mount point
sudo mkdir -p /mnt/tokitai

# Run the FUSE filesystem (as regular user, no sudo needed)
cargo run --features fuse --bin tokitai-fuse -- /mnt/tokitai
```

### Accessing Data

```bash
# List column families (directories)
ls /mnt/tokitai/

# Create a new file (key)
echo "value" > /mnt/tokitai/default/mykey

# Read a file (get value)
cat /mnt/tokitai/default/mykey

# Create directory (namespace)
mkdir /mnt/tokitai/default/users

# Delete file
rm /mnt/tokitai/default/mykey
```

## API Reference

### FuseConfig

```rust
pub struct FuseConfig {
    pub mount_point: PathBuf,
    pub root_path: PathBuf,
    pub debug: bool,
    pub allow_other: bool,
    pub auto_unmount: bool,
}
```

#### Configuration Options

```rust
// Basic config
let config = FuseConfig::new("/mnt/tokitai");

// With custom root
let config = config.with_root_path("/var/lib/tokitai");

// Enable debug
let config = config.with_debug(true);
```

### FuseFS

```rust
pub struct FuseFS {
    // Internal state
}
```

#### Methods

```rust
// Create new FUSE filesystem
let fs = FuseFS::new(config);

// Mount (prepare mount options)
fs.mount()?;

// Run (blocking, handles FUSE requests)
fs.run()?;

// Unmount
fs.unmount()?;
```

## Filesystem Layout

```
/mnt/tokitai/
├── default/           # Default column family
│   ├── key1           # Key-value pairs as files
│   ├── key2
│   └── namespace/     # Nested namespaces
│       └── key3
├── users/             # Users column family
│   ├── user:1
│   └── user:2
└── sessions/          # Sessions column family
    └── session:abc123
```

## FUSE Operations

### Implemented Operations

| Operation | Description | Status |
|-----------|-------------|--------|
| `lookup` | Look up file by name | ✅ |
| `getattr` | Get file attributes | ✅ |
| `setattr` | Set file attributes | ✅ |
| `open` | Open a file | ✅ |
| `read` | Read file data | ✅ |
| `write` | Write file data | ✅ |
| `release` | Close open file | ✅ |
| `create` | Create new file | ✅ |
| `unlink` | Delete file | ✅ |
| `mkdir` | Create directory | ✅ |
| `rmdir` | Remove directory | ✅ |
| `readdir` | Read directory | ✅ |
| `utimens` | Update timestamps | ✅ |

### File Attributes

```rust
pub struct InodeAttr {
    pub ino: u64,      // Inode number
    pub size: u64,     // File size
    pub mode: u32,     // File mode (permissions)
    pub nlink: u32,    // Link count
    pub uid: u32,      // Owner UID
    pub gid: u32,      // Owner GID
    pub atime: SystemTime,  // Access time
    pub mtime: SystemTime,  // Modify time
    pub ctime: SystemTime,  // Change time
}
```

## Error Handling

### FuseError Types

```rust
pub enum FuseError {
    Io(std::io::Error),
    NotAvailable(String),
    InvalidPath(String),
    NotFound(String),
    AlreadyExists(String),
    PermissionDenied(String),
    ColumnFamily(ColumnFamilyError),
    NotADirectory(String),
    DirectoryNotEmpty(String),
}
```

### Error Codes

| Error | errno |
|-------|-------|
| NotFound | ENOENT (2) |
| AlreadyExists | EEXIST (17) |
| PermissionDenied | EACCES (13) |
| NotADirectory | ENOTDIR (20) |
| DirectoryNotEmpty | ENOTEMPTY (39) |
| NotAvailable | ENOSYS (38) |

## Testing

### Unit Tests

```bash
# Run unit tests (no FUSE mount required)
cargo test --lib fuse_fs::tests
```

### Integration Test (requires FUSE)

```bash
# Create mount point
mkdir -p /tmp/test_tokitai

# Run FUSE in background
cargo run --features fuse --bin tokitai-fuse -- /tmp/test_tokitai &
FUSE_PID=$!

# Test operations
echo "test_value" > /tmp/test_tokitai/default/test_key
cat /tmp/test_tokitai/default/test_key

# Cleanup
kill $FUSE_PID
fusermount -u /tmp/test_tokitai
```

## Mount Options

### allow_other

Allow other users to access the mount:

```rust
let config = FuseConfig::new("/mnt/tokitai")
    .with_allow_other(true);
```

Note: Requires `user_allow_other` in `/etc/fuse.conf`.

### debug

Enable FUSE debug logging:

```rust
let config = FuseConfig::new("/mnt/tokitai")
    .with_debug(true);
```

### auto_unmount

Automatically unmount when the process exits:

```rust
// Enabled by default
let config = FuseConfig::new("/mnt/tokitai");
```

## Performance Considerations

### Caching

FUSE kernel module caches attributes and directory entries. Tune with:

```rust
// In kernel mount options (not in userspace)
mount -t fuse.tokitai tokitai-fuse /mnt/tokitai -o entry_timeout=60,attr_timeout=60
```

### Buffer Sizes

Default read/write buffer is 4KB. For large files:

```rust
// In fuser, use large buffer size
// Note: This requires fuser configuration
```

### Batch Operations

For multiple file operations, consider:

```bash
# Use tar for bulk operations
tar -cf backup.tar /mnt/tokitai/default/
```

## Limitations

1. **No Hard Links**: Each file has exactly one path
2. **No Symlinks**: Symbolic links not supported
3. **No File Locks**: Advisory locks not implemented
4. **No Extended Attributes**: xattr not supported
5. **No ACLs**: Access control lists not supported

## Troubleshooting

### Mount Failed: Permission Denied

```bash
# Check FUSE permissions
ls -la /dev/fuse

# Add user to fuse group
sudo usermod -a -G fuse $USER

# Reload groups
newgrp fuse
```

### Mount Point Busy

```bash
# Check what's using the mount point
lsof +D /mnt/tokitai

# Force unmount (use with caution)
fusermount -uz /mnt/tokitai
```

### Debug Mode

```bash
# Run with debug output
RUST_LOG=debug cargo run --features fuse -- /mnt/tokitai

# Enable FUSE kernel debug
mount -t fuse.tokitai -o debug tokitai-fuse /mnt/tokitai
```

## Security Considerations

### Mount Permissions

By default, only the mounting user can access the filesystem. To allow other users:

1. Add `user_allow_other` to `/etc/fuse.conf`
2. Use `allow_other` mount option

### Data Protection

FUSE filesystem runs in userspace, so:
- File data is not encrypted at rest
- Use filesystem-level encryption if needed
- Consider using encrypted column families

## Integration Examples

### Systemd Service

```ini
# /etc/systemd/system/tokitai-fuse.service
[Unit]
Description=Tokitai FUSE Filesystem
After=network.target

[Service]
Type=simple
User=tokitai
ExecStart=/usr/bin/tokitai-fuse /mnt/tokitai
Restart=always

[Install]
WantedBy=multi-user.target
```

### fstab Entry

```
# /etc/fstab
tokitai-fuse /mnt/tokitai fuse defaults,allow_other 0 0
```

## Future Enhancements

- [ ] Symlink support
- [ ] Hard link support
- [ ] Extended attributes (xattr)
- [ ] File locking (flock/fcntl)
- [ ] Access control lists (ACLs)
- [ ] NFS export
- [ ] Compression at filesystem level
- [ ] Encryption at filesystem level

## Related Modules

- **P3-005**: Column Family - Data organization
- **P3-004**: Distributed Coordination - Multi-node support
- **P2-004**: Block Cache - Performance optimization

## References

- [FUSE Documentation](https://www.kernel.org/doc/html/latest/filesystems/fuse.html)
- [fuser Crate](https://docs.rs/fuser/)
- [libfuse GitHub](https://github.com/libfuse/libfuse)
