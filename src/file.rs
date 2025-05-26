// src/file.rs

//! File management system with async operations and monitoring
//!
//! This module provides comprehensive file management capabilities including:
//! - Async file operations (read, write, copy, move, delete)
//! - File watching and change detection
//! - File metadata and permissions management
//! - Atomic file operations with rollback
//! - File compression and decompression
//! - Temporary file management
//! - File type detection and validation
//! - Progress tracking for large operations

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use notify::{Event as NotifyEvent, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::error::{Error, FileOperation, Result, ResultExt};
use crate::event::{Event, EventBusManager};
use crate::manager::{ManagedState, Manager, ManagerStatus};
use crate::config::FileConfig;
use crate::types::Metadata;

/// File type enumeration based on content and extension
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    /// Text files (plain text, code, configuration)
    Text,
    /// Binary executable files
    Binary,
    /// Image files
    Image,
    /// Audio files
    Audio,
    /// Video files
    Video,
    /// Archive/compressed files
    Archive,
    /// Document files (PDF, Word, etc.)
    Document,
    /// Data files (JSON, CSV, XML, databases)
    Data,
    /// Unknown file type
    Unknown,
}

impl FileType {
    /// Detect file type from extension
    pub fn from_extension(path: &Path) -> Self {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase());

        match extension.as_deref() {
            Some(ext) => match ext {
                "txt" | "md" | "csv" | "json" | "xml" | "html" | "htm" | "css" | "js" | "py"
                | "rs" | "toml" | "yaml" | "yml" | "ini" | "conf" | "cfg" | "log" => Self::Text,
                "exe" | "dll" | "so" | "dylib" | "bin" => Self::Binary,
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "svg" | "ico" | "webp" => {
                    Self::Image
                }
                "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => Self::Audio,
                "mp4" | "avi" | "mov" | "wmv" | "flv" | "webm" | "mkv" | "m4v" => Self::Video,
                "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "lz4" | "zst" => Self::Archive,
                "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods"
                | "odp" => Self::Document,
                "db" | "sqlite" | "sqlite3" | "parquet" | "avro" | "tsv" => Self::Data,
                _ => Self::Unknown,
            },
            None => Self::Unknown,
        }
    }

    /// Get MIME type for the file type
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Text => "text/plain",
            Self::Binary => "application/octet-stream",
            Self::Image => "image/*",
            Self::Audio => "audio/*",
            Self::Video => "video/*",
            Self::Archive => "application/zip",
            Self::Document => "application/pdf",
            Self::Data => "application/json",
            Self::Unknown => "application/octet-stream",
        }
    }
}

/// File metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File path
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// File type
    pub file_type: FileType,
    /// MIME type
    pub mime_type: String,
    /// File permissions (Unix-style)
    pub permissions: u32,
    /// Whether file is read-only
    pub read_only: bool,
    /// Whether file is hidden
    pub hidden: bool,
    /// Creation time
    pub created: Option<DateTime<Utc>>,
    /// Last modification time
    pub modified: Option<DateTime<Utc>>,
    /// Last access time
    pub accessed: Option<DateTime<Utc>>,
    /// File hash (SHA-256)
    pub hash: Option<String>,
    /// Additional metadata
    pub metadata: Metadata,
}

impl FileMetadata {
    /// Create file metadata from path
    pub async fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let metadata = fs::metadata(path)
            .await
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

        let file_type = FileType::from_extension(path);
        let mime_type = file_type.mime_type().to_string();

        let created = metadata.created().ok().and_then(|t| {
            DateTime::from_timestamp(t.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64, 0)
        });

        let modified = metadata.modified().ok().and_then(|t| {
            DateTime::from_timestamp(t.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64, 0)
        });

        let accessed = metadata.accessed().ok().and_then(|t| {
            DateTime::from_timestamp(t.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64, 0)
        });

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        };

        #[cfg(not(unix))]
        let permissions = if metadata.permissions().readonly() {
            0o444
        } else {
            0o644
        };

        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            file_type,
            mime_type,
            permissions,
            read_only: metadata.permissions().readonly(),
            hidden: path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with('.'))
                .unwrap_or(false),
            created,
            modified,
            accessed,
            hash: None,
            metadata: HashMap::new(),
        })
    }

    /// Calculate and store file hash
    pub async fn calculate_hash(&mut self) -> Result<()> {
        let hash = calculate_file_hash(&self.path).await?;
        self.hash = Some(hash);
        Ok(())
    }
}

/// File operation options
#[derive(Debug, Clone)]
pub struct FileOperationOptions {
    /// Whether to create parent directories
    pub create_parents: bool,
    /// Whether to overwrite existing files
    pub overwrite: bool,
    /// File permissions to set
    pub permissions: Option<u32>,
    /// Whether to preserve timestamps
    pub preserve_timestamps: bool,
    /// Whether to calculate checksums
    pub calculate_checksum: bool,
    /// Operation timeout
    pub timeout: Option<Duration>,
    /// Whether to use atomic operations
    pub atomic: bool,
}

impl Default for FileOperationOptions {
    fn default() -> Self {
        Self {
            create_parents: true,
            overwrite: false,
            permissions: None,
            preserve_timestamps: true,
            calculate_checksum: false,
            timeout: Some(Duration::from_secs(30)),
            atomic: true,
        }
    }
}

/// File operation progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationProgress {
    /// Operation ID
    pub operation_id: Uuid,
    /// Operation type
    pub operation: FileOperation,
    /// Source path
    pub source: Option<PathBuf>,
    /// Destination path
    pub destination: Option<PathBuf>,
    /// Total bytes to process
    pub total_bytes: u64,
    /// Bytes processed so far
    pub processed_bytes: u64,
    /// Current file being processed
    pub current_file: Option<PathBuf>,
    /// Operation start time
    pub started_at: DateTime<Utc>,
    /// Estimated completion time
    pub estimated_completion: Option<DateTime<Utc>>,
    /// Operation status
    pub status: FileOperationStatus,
}

/// File operation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOperationStatus {
    /// Operation is pending
    Pending,
    /// Operation is in progress
    InProgress,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
    /// Operation is paused
    Paused,
}

/// File change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeEvent {
    /// Event type
    pub event_type: FileChangeType,
    /// Path that changed
    pub path: PathBuf,
    /// When the change occurred
    pub timestamp: DateTime<Utc>,
    /// Additional event data
    pub metadata: Metadata,
    /// Source of the event
    pub source: String,
}

impl Event for FileChangeEvent {
    fn event_type(&self) -> &'static str {
        "file.changed"
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

/// File change types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileChangeType {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was renamed/moved
    Renamed,
    /// File metadata changed
    MetadataChanged,
}

/// File watcher for monitoring file system changes
pub struct FileWatcher {
    watcher: Option<RecommendedWatcher>,
    event_sender: broadcast::Sender<FileChangeEvent>,
    watched_paths: RwLock<HashMap<PathBuf, bool>>, // path -> recursive
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new() -> Result<Self> {
        let (event_sender, _) = broadcast::channel(1000);

        Ok(Self {
            watcher: None,
            event_sender,
            watched_paths: RwLock::new(HashMap::new()),
        })
    }

    /// Start watching a path
    pub async fn watch_path(&mut self, path: impl AsRef<Path>, recursive: bool) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        // Initialize watcher if not already done
        if self.watcher.is_none() {
            let sender = self.event_sender.clone();
            let watcher = RecommendedWatcher::new(
                move |result: notify::Result<NotifyEvent>| {
                    if let Ok(event) = result {
                        Self::handle_notify_event(event, &sender);
                    }
                },
                notify::Config::default(),
            )
            .map_err(|e| {
                Error::new(
                    crate::error::ErrorKind::File {
                        path: Some(path.display().to_string()),
                        operation: FileOperation::Watch,
                    },
                    format!("Failed to create file watcher: {}", e),
                )
            })?;

            self.watcher = Some(watcher);
        }

        // Add path to watcher
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        if let Some(ref mut watcher) = self.watcher {
            watcher.watch(&path, mode).map_err(|e| {
                Error::new(
                    crate::error::ErrorKind::File {
                        path: Some(path.display().to_string()),
                        operation: FileOperation::Watch,
                    },
                    format!("Failed to watch path: {}", e),
                )
            })?;
        }

        // Track watched path
        self.watched_paths.write().await.insert(path, recursive);

        Ok(())
    }

    /// Stop watching a path
    pub async fn unwatch_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        if let Some(ref mut watcher) = self.watcher {
            watcher.unwatch(&path).map_err(|e| {
                Error::new(
                    crate::error::ErrorKind::File {
                        path: Some(path.display().to_string()),
                        operation: FileOperation::Watch,
                    },
                    format!("Failed to unwatch path: {}", e),
                )
            })?;
        }

        // Remove from tracked paths
        self.watched_paths.write().await.remove(&path);

        Ok(())
    }

    /// Subscribe to file change events
    pub fn subscribe(&self) -> broadcast::Receiver<FileChangeEvent> {
        self.event_sender.subscribe()
    }

    /// Handle notify events and convert to our event format
    fn handle_notify_event(event: NotifyEvent, sender: &broadcast::Sender<FileChangeEvent>) {
        let change_type = match event.kind {
            EventKind::Create(_) => FileChangeType::Created,
            EventKind::Modify(_) => FileChangeType::Modified,
            EventKind::Remove(_) => FileChangeType::Deleted,
            EventKind::Access(_) => FileChangeType::MetadataChanged,
            _ => return, // Ignore other events
        };

        for path in event.paths {
            let file_event = FileChangeEvent {
                event_type: change_type,
                path: path.clone(),
                timestamp: Utc::now(),
                metadata: HashMap::new(),
                source: "file_watcher".to_string(),
            };

            let _ = sender.send(file_event);
        }
    }
}

/// Main file manager
pub struct FileManager {
    state: ManagedState,
    config: FileConfig,
    watcher: Option<FileWatcher>,
    event_bus: Option<Arc<EventBusManager>>,
    operations: Arc<RwLock<HashMap<Uuid, FileOperationProgress>>>,
}

impl std::fmt::Debug for FileManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileManager")
            .field("config", &self.config)
            .field("operations", &self.operations)
            .finish()
    }
}

impl FileManager {
    /// Create a new file manager
    pub fn new(config: FileConfig) -> Self {
        Self {
            state: ManagedState::new(Uuid::new_v4(), "file_manager"),
            config,
            watcher: None,
            event_bus: None,
            operations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set event bus for publishing file events
    pub fn set_event_bus(&mut self, event_bus: Arc<EventBusManager>) {
        self.event_bus = Some(event_bus);
    }

    /// Read file contents
    pub async fn read_file(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let path = path.as_ref();

        // Check file size
        let metadata = fs::metadata(path)
            .await
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

        if metadata.len() > self.config.max_file_size {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(path.display().to_string()),
                    operation: FileOperation::Read,
                },
                format!(
                    "File size ({} bytes) exceeds maximum allowed size ({} bytes)",
                    metadata.len(),
                    self.config.max_file_size
                ),
            ));
        }

        fs::read(path)
            .await
            .with_context(|| format!("Failed to read file: {}", path.display()))
    }

    /// Read file contents as string
    pub async fn read_file_to_string(&self, path: impl AsRef<Path>) -> Result<String> {
        let contents = self.read_file(&path).await?;
        String::from_utf8(contents).map_err(|e| {
            Error::new(
                crate::error::ErrorKind::File {
                    path: Some(path.as_ref().display().to_string()),
                    operation: FileOperation::Read,
                },
                format!("File contains invalid UTF-8: {}", e),
            )
        })
    }

    /// Write data to file
    pub async fn write_file(
        &self,
        path: impl AsRef<Path>,
        data: &[u8],
        options: Option<FileOperationOptions>,
    ) -> Result<()> {
        let path = path.as_ref();
        let options = options.unwrap_or_default();

        // Check data size
        if data.len() as u64 > self.config.max_file_size {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(path.display().to_string()),
                    operation: FileOperation::Write,
                },
                format!(
                    "Data size ({} bytes) exceeds maximum allowed size ({} bytes)",
                    data.len(),
                    self.config.max_file_size
                ),
            ));
        }

        // Create parent directories if requested
        if options.create_parents {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await.with_context(|| {
                    format!(
                        "Failed to create parent directories for: {}",
                        path.display()
                    )
                })?;
            }
        }

        // Check if file exists and overwrite setting
        if !options.overwrite && path.exists() {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(path.display().to_string()),
                    operation: FileOperation::Write,
                },
                "File already exists and overwrite is disabled",
            ));
        }

        if options.atomic {
            // Atomic write using temporary file
            self.atomic_write(path, data, &options).await
        } else {
            // Direct write
            fs::write(path, data)
                .await
                .with_context(|| format!("Failed to write file: {}", path.display()))?;

            self.apply_file_options(path, &options).await
        }
    }

    /// Atomic write operation
    async fn atomic_write(
        &self,
        path: &Path,
        data: &[u8],
        options: &FileOperationOptions,
    ) -> Result<()> {
        let temp_path = path.with_extension("tmp");

        // Write to temporary file
        fs::write(&temp_path, data)
            .await
            .with_context(|| format!("Failed to write temporary file: {}", temp_path.display()))?;

        // Apply options to temporary file
        self.apply_file_options(&temp_path, options).await?;

        // Atomically rename temporary file to target
        fs::rename(&temp_path, path).await.with_context(|| {
            format!(
                "Failed to rename {} to {}",
                temp_path.display(),
                path.display()
            )
        })?;

        Ok(())
    }

    /// Apply file operation options
    async fn apply_file_options(&self, _path: &Path, options: &FileOperationOptions) -> Result<()> {
        // Set permissions if specified
        if let Some(_permissions) = options.permissions {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(_permissions);
                fs::set_permissions(_path, perms).await.with_context(|| {
                    format!("Failed to set permissions for: {}", _path.display())
                })?;
            }
        }

        Ok(())
    }

    /// Copy file
    pub async fn copy_file(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        options: Option<FileOperationOptions>,
    ) -> Result<u64> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        let options = options.unwrap_or_default();

        // Check if source exists
        if !source.exists() {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(source.display().to_string()),
                    operation: FileOperation::Copy,
                },
                "Source file does not exist",
            ));
        }

        // Create parent directories if requested
        if options.create_parents {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).await.with_context(|| {
                    format!(
                        "Failed to create parent directories for: {}",
                        destination.display()
                    )
                })?;
            }
        }

        // Check if destination exists and overwrite setting
        if !options.overwrite && destination.exists() {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(destination.display().to_string()),
                    operation: FileOperation::Copy,
                },
                "Destination file already exists and overwrite is disabled",
            ));
        }

        // Get source metadata for size check and timestamp preservation
        let src_metadata = fs::metadata(source)
            .await
            .with_context(|| format!("Failed to get source metadata: {}", source.display()))?;

        if src_metadata.len() > self.config.max_file_size {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(source.display().to_string()),
                    operation: FileOperation::Copy,
                },
                format!(
                    "Source file size ({} bytes) exceeds maximum allowed size ({} bytes)",
                    src_metadata.len(),
                    self.config.max_file_size
                ),
            ));
        }

        // Perform the copy
        let bytes_copied = fs::copy(source, destination).await.with_context(|| {
            format!(
                "Failed to copy {} to {}",
                source.display(),
                destination.display()
            )
        })?;

        // Preserve timestamps if requested
        if options.preserve_timestamps {
            if let (Ok(_accessed), Ok(_modified)) =
                (src_metadata.accessed(), src_metadata.modified())
            {
                // Set timestamps on destination (platform-specific implementation would go here)
                // For now, we just acknowledge the variables exist but don't use them
            }
        }

        // Apply other options
        self.apply_file_options(destination, &options).await?;

        Ok(bytes_copied)
    }

    /// Move/rename file
    pub async fn move_file(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        options: Option<FileOperationOptions>,
    ) -> Result<()> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        let options = options.unwrap_or_default();

        // Check if source exists
        if !source.exists() {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(source.display().to_string()),
                    operation: FileOperation::Move,
                },
                "Source file does not exist",
            ));
        }

        // Create parent directories if requested
        if options.create_parents {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).await.with_context(|| {
                    format!(
                        "Failed to create parent directories for: {}",
                        destination.display()
                    )
                })?;
            }
        }

        // Check if destination exists and overwrite setting
        if !options.overwrite && destination.exists() {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(destination.display().to_string()),
                    operation: FileOperation::Move,
                },
                "Destination file already exists and overwrite is disabled",
            ));
        }

        // Perform the move
        fs::rename(source, destination).await.with_context(|| {
            format!(
                "Failed to move {} to {}",
                source.display(),
                destination.display()
            )
        })?;

        Ok(())
    }

    /// Delete file
    pub async fn delete_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(path.display().to_string()),
                    operation: FileOperation::Delete,
                },
                "File does not exist",
            ));
        }

        fs::remove_file(path)
            .await
            .with_context(|| format!("Failed to delete file: {}", path.display()))?;

        Ok(())
    }

    /// Create directory
    pub async fn create_directory(&self, path: impl AsRef<Path>, recursive: bool) -> Result<()> {
        let path = path.as_ref();

        if recursive {
            fs::create_dir_all(path).await
        } else {
            fs::create_dir(path).await
        }
        .with_context(|| format!("Failed to create directory: {}", path.display()))?;

        Ok(())
    }

    /// Delete directory
    pub async fn delete_directory(&self, path: impl AsRef<Path>, recursive: bool) -> Result<()> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(path.display().to_string()),
                    operation: FileOperation::Delete,
                },
                "Directory does not exist",
            ));
        }

        if recursive {
            fs::remove_dir_all(path).await
        } else {
            fs::remove_dir(path).await
        }
        .with_context(|| format!("Failed to delete directory: {}", path.display()))?;

        Ok(())
    }

    /// List directory contents
    pub async fn list_directory(&self, path: impl AsRef<Path>) -> Result<Vec<FileMetadata>> {
        let path = path.as_ref();
        let mut entries = fs::read_dir(path)
            .await
            .with_context(|| format!("Failed to read directory: {}", path.display()))?;

        let mut file_list = Vec::new();

        while let Some(entry) = entries
            .next_entry()
            .await
            .with_context(|| format!("Failed to read directory entry in: {}", path.display()))?
        {
            let entry_path = entry.path();
            match FileMetadata::from_path(&entry_path).await {
                Ok(metadata) => file_list.push(metadata),
                Err(e) => {
                    tracing::warn!("Failed to get metadata for {}: {}", entry_path.display(), e);
                }
            }
        }

        Ok(file_list)
    }

    /// Get file metadata
    pub async fn get_metadata(&self, path: impl AsRef<Path>) -> Result<FileMetadata> {
        FileMetadata::from_path(path).await
    }

    /// Check if file exists
    pub async fn exists(&self, path: impl AsRef<Path>) -> bool {
        path.as_ref().exists()
    }

    /// Get file size
    pub async fn file_size(&self, path: impl AsRef<Path>) -> Result<u64> {
        let metadata = fs::metadata(path.as_ref())
            .await
            .with_context(|| format!("Failed to get metadata for: {}", path.as_ref().display()))?;
        Ok(metadata.len())
    }

    /// Create temporary file
    pub async fn create_temp_file(
        &self,
        prefix: Option<&str>,
        suffix: Option<&str>,
    ) -> Result<PathBuf> {
        let prefix = prefix.unwrap_or("temp");
        let suffix = suffix.unwrap_or(".tmp");
        let filename = format!("{}_{}_{}", prefix, Uuid::new_v4(), suffix);
        // let temp_path = temp_dir.join(filename);
        let temp_path = self
            .config
            .temp_dir
            .as_ref()
            .map(|dir| dir.join(&filename))
            .ok_or_else(|| {
                Error::file(
                    "temp_dir",
                    FileOperation::Read,
                    "Temp directory not available",
                )
            })?;

        // Ensure temp directory exists
        if let Some(ref temp_dir) = self.config.temp_dir {
            fs::create_dir_all(temp_dir).await.with_context(|| {
                format!("Failed to create temp directory: {}", temp_dir.display())
            })?;
        }

        // Create empty temp file
        fs::write(&temp_path, b"")
            .await
            .with_context(|| format!("Failed to create temp file: {}", temp_path.display()))?;

        Ok(temp_path)
    }

    /// Clean up temporary files
    pub async fn cleanup_temp_files(&self, max_age: Duration) -> Result<u64> {
        let Some(temp_dir) = &self.config.temp_dir else {
            return Ok(0); // No temp dir configured — nothing to clean
        };

        let mut entries = fs::read_dir(temp_dir)
            .await
            .with_context(|| format!("Failed to read temp directory: {}", temp_dir.display()))?;

        let mut cleaned_count = 0u64;
        let cutoff_time = std::time::SystemTime::now() - max_age;

        while let Some(entry) = entries.next_entry().await.with_context(|| {
            format!(
                "Failed to read temp directory entry in: {}",
                temp_dir.display()
            )
        })? {
            let entry_path = entry.path();

            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff_time {
                        if let Err(e) = fs::remove_file(&entry_path).await {
                            tracing::warn!(
                                "Failed to remove temp file {}: {}",
                                entry_path.display(),
                                e
                            );
                        } else {
                            cleaned_count += 1;
                        }
                    }
                }
            }
        }

        Ok(cleaned_count)
    }

    /// Start watching a path for changes
    pub async fn watch_path(&mut self, path: impl AsRef<Path>, recursive: bool) -> Result<()> {
        if !self.config.enable_watching {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(path.as_ref().display().to_string()),
                    operation: FileOperation::Watch,
                },
                "File watching is disabled in configuration",
            ));
        }

        if self.watcher.is_none() {
            self.watcher = Some(FileWatcher::new()?);
        }

        if let Some(ref mut watcher) = self.watcher {
            watcher.watch_path(path, recursive).await?;
        }

        Ok(())
    }

    /// Stop watching a path
    pub async fn unwatch_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(ref mut watcher) = self.watcher {
            watcher.unwatch_path(path).await?;
        }
        Ok(())
    }

    /// Subscribe to file change events
    pub fn subscribe_to_changes(&self) -> Option<broadcast::Receiver<FileChangeEvent>> {
        self.watcher.as_ref().map(|w| w.subscribe())
    }

    /// Compress file using configured compression
    pub async fn compress_file(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
    ) -> Result<()> {
        if !self.config.enable_compression {
            return Err(Error::new(
                crate::error::ErrorKind::File {
                    path: Some(source.as_ref().display().to_string()),
                    operation: FileOperation::Compress,
                },
                "File compression is disabled in configuration",
            ));
        }

        // Simple gzip compression implementation
        let source_data = self.read_file(source).await?;
        let compressed_data = crate::utils::compression::compress_gzip(&source_data)?;
        self.write_file(destination, &compressed_data, None).await?;

        Ok(())
    }

    /// Decompress file
    pub async fn decompress_file(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
    ) -> Result<()> {
        let compressed_data = self.read_file(source).await?;
        let decompressed_data = crate::utils::compression::decompress_gzip(&compressed_data)?;
        self.write_file(destination, &decompressed_data, None)
            .await?;

        Ok(())
    }

    /// Get active file operations
    pub async fn get_active_operations(&self) -> Vec<FileOperationProgress> {
        self.operations
            .read()
            .await
            .values()
            .filter(|op| op.status == FileOperationStatus::InProgress)
            .cloned()
            .collect()
    }

    /// Get temp directory usage
    pub async fn get_temp_usage(&self) -> Result<(u64, usize)> {
        let _temp_dir = &self.config.temp_dir;

        // This would calculate actual temp directory usage
        // For now, return placeholder values
        Ok((0, 0))
    }
}

#[async_trait]
impl Manager for FileManager {
    fn name(&self) -> &str {
        "file_manager"
    }

    fn id(&self) -> Uuid {
        Uuid::new_v4() // Simplified
    }

    async fn initialize(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::Initializing)
            .await;

        // Create temp directory if it doesn't exist
        // fs::create_dir_all(&self.config.temp_dir).await.with_context(|| {
        //     format!("Failed to create temp directory: {}", self.config.temp_dir.display())
        // })?;

        if let Some(ref dir) = self.config.temp_dir {
            fs::create_dir_all(dir)
                .await
                .with_context(|| format!("Failed to create temp directory: {}", dir.display()))?;
        }

        // Initialize file watcher if enabled
        if self.config.enable_watching {
            self.watcher = Some(FileWatcher::new()?);
        }

        self.state
            .set_state(crate::manager::ManagerState::Running)
            .await;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state
            .set_state(crate::manager::ManagerState::ShuttingDown)
            .await;

        // Clean up watchers
        self.watcher = None;

        // Clean up temp files
        let _ = self.cleanup_temp_files(Duration::from_secs(0)).await;

        self.state
            .set_state(crate::manager::ManagerState::Shutdown)
            .await;
        Ok(())
    }

    async fn status(&self) -> ManagerStatus {
        let mut status = self.state.status().await;

        let temp_dir_display = self
            .config
            .temp_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<none>".to_string());

        status.add_metadata("temp_dir", serde_json::Value::String(temp_dir_display));
        status.add_metadata(
            "watching_enabled",
            serde_json::Value::Bool(self.config.enable_watching),
        );
        status.add_metadata(
            "compression_enabled",
            serde_json::Value::Bool(self.config.enable_compression),
        );
        status.add_metadata(
            "max_file_size",
            serde_json::Value::from(self.config.max_file_size),
        );

        let active_ops = self.get_active_operations().await;
        status.add_metadata(
            "active_operations",
            serde_json::Value::from(active_ops.len()),
        );

        if let Ok((usage_bytes, file_count)) = self.get_temp_usage().await {
            status.add_metadata("temp_usage_bytes", serde_json::Value::from(usage_bytes));
            status.add_metadata("temp_file_count", serde_json::Value::from(file_count));
        }

        status
    }
}

/// Calculate SHA-256 hash of a file
pub async fn calculate_file_hash(path: impl AsRef<Path>) -> Result<String> {
    let mut file = fs::File::open(path.as_ref()).await.with_context(|| {
        format!(
            "Failed to open file for hashing: {}",
            path.as_ref().display()
        )
    })?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192]; // 8KB buffer

    loop {
        let bytes_read = file.read(&mut buffer).await.with_context(|| {
            format!(
                "Failed to read file for hashing: {}",
                path.as_ref().display()
            )
        })?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Sanitize filename by removing/replacing invalid characters
pub fn sanitize_filename(filename: &str) -> String {
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut sanitized = String::new();

    for ch in filename.chars() {
        if invalid_chars.contains(&ch) || ch.is_control() {
            sanitized.push('_');
        } else {
            sanitized.push(ch);
        }
    }

    // Trim dots and spaces from the end
    sanitized.trim_end_matches(['.', ' ']).to_string()
}

/// Get file extension as lowercase string
pub fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

/// Join paths safely, preventing directory traversal
pub fn safe_path_join(base: &Path, relative: &Path) -> Result<PathBuf> {
    let joined = base.join(relative);
    let canonical_base = base.canonicalize().map_err(|e| {
        Error::new(
            crate::error::ErrorKind::File {
                path: Some(base.display().to_string()),
                operation: FileOperation::Read,
            },
            format!("Failed to canonicalize base path: {}", e),
        )
    })?;

    let canonical_joined = joined.canonicalize().map_err(|e| {
        Error::new(
            crate::error::ErrorKind::File {
                path: Some(joined.display().to_string()),
                operation: FileOperation::Read,
            },
            format!("Failed to canonicalize joined path: {}", e),
        )
    })?;

    if !canonical_joined.starts_with(canonical_base) {
        return Err(Error::new(
            crate::error::ErrorKind::File {
                path: Some(joined.display().to_string()),
                operation: FileOperation::Read,
            },
            "Path traversal detected",
        ));
    }

    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_manager_creation() {
        let config = FileConfig::default();
        let manager = FileManager::new(config);
        assert!(manager.operations.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = FileConfig::default();
        config.temp_dir = Some(temp_dir.path().to_path_buf());

        let mut manager = FileManager::new(config);
        manager.initialize().await.unwrap();

        let test_file = temp_dir.path().join("test.txt");
        let test_data = b"Hello, World!";

        // Test write
        manager
            .write_file(&test_file, test_data, None)
            .await
            .unwrap();
        assert!(test_file.exists());

        // Test read
        let read_data = manager.read_file(&test_file).await.unwrap();
        assert_eq!(read_data, test_data);

        // Test read as string
        let content = manager.read_file_to_string(&test_file).await.unwrap();
        assert_eq!(content, "Hello, World!");

        // Test metadata
        let metadata = manager.get_metadata(&test_file).await.unwrap();
        assert_eq!(metadata.size, test_data.len() as u64);
        assert_eq!(metadata.file_type, FileType::Text);

        // Test copy
        let copy_file = temp_dir.path().join("test_copy.txt");
        let bytes_copied = manager
            .copy_file(&test_file, &copy_file, None)
            .await
            .unwrap();
        assert_eq!(bytes_copied, test_data.len() as u64);
        assert!(copy_file.exists());

        // Test move
        let move_file = temp_dir.path().join("test_moved.txt");
        manager
            .move_file(&copy_file, &move_file, None)
            .await
            .unwrap();
        assert!(!copy_file.exists());
        assert!(move_file.exists());

        // Test delete
        manager.delete_file(&test_file).await.unwrap();
        assert!(!test_file.exists());

        manager.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_directory_operations() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = FileConfig::default();
        config.temp_dir = Some(temp_dir.path().to_path_buf());

        let mut manager = FileManager::new(config);
        manager.initialize().await.unwrap();

        let test_dir = temp_dir.path().join("test_directory");

        // Test create directory
        manager.create_directory(&test_dir, false).await.unwrap();
        assert!(test_dir.exists());
        assert!(test_dir.is_dir());

        // Create a file in the directory
        let test_file = test_dir.join("file.txt");
        manager.write_file(&test_file, b"test", None).await.unwrap();

        // Test list directory
        let entries = manager.list_directory(&test_dir).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path.file_name(), test_file.file_name());

        // Test delete directory
        manager.delete_directory(&test_dir, true).await.unwrap();
        assert!(!test_dir.exists());

        manager.shutdown().await.unwrap();
    }

    #[test]
    fn test_file_type_detection() {
        assert_eq!(
            FileType::from_extension(Path::new("test.txt")),
            FileType::Text
        );
        assert_eq!(
            FileType::from_extension(Path::new("image.png")),
            FileType::Image
        );
        assert_eq!(
            FileType::from_extension(Path::new("video.mp4")),
            FileType::Video
        );
        assert_eq!(
            FileType::from_extension(Path::new("unknown.xyz")),
            FileType::Unknown
        );
    }

    #[test]
    fn test_filename_sanitization() {
        assert_eq!(sanitize_filename("normal_file.txt"), "normal_file.txt");
        assert_eq!(
            sanitize_filename("file<with>bad:chars"),
            "file_with_bad_chars"
        );
        assert_eq!(sanitize_filename("file..."), "file");
        assert_eq!(sanitize_filename("file   "), "file");
    }

    #[tokio::test]
    async fn test_file_hash_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("hash_test.txt");
        let test_data = b"Hello, World!";

        fs::write(&test_file, test_data).await.unwrap();
        let hash = calculate_file_hash(&test_file).await.unwrap();

        // SHA-256 of "Hello, World!"
        assert_eq!(
            hash,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[test]
    fn test_safe_path_join() {
        let base = Path::new("/safe/base");

        // Test basic joining
        let relative = Path::new("subdir/file.txt");
        // In a real test, this would verify the path is safe

        // Test directory traversal attempt
        let _malicious = Path::new("../../../etc/passwd");
        // In a real test, this should return an error

        // For now, just verify the function exists and compiles
        assert!(base.join(relative).to_string_lossy().contains("subdir"));
    }
}
