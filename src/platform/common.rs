use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[cfg(windows)]
use std::fs::OpenOptions;

/// Atomically write content to a file
///
/// This function writes to a temporary file in the same directory,
/// syncs to disk, then renames to the target path. This ensures
/// the write is atomic on Unix and NTFS filesystems.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        ensure_directory_exists(parent)?;
    }

    // Create temporary file in same directory
    let temp_path = path.with_extension("tmp");

    {
        let mut file = File::create(&temp_path).with_context(|| {
            format!("Failed to create temporary file: {}", temp_path.display())
        })?;

        file.write_all(content)
            .context("Failed to write to temporary file")?;

        file.sync_all().context("Failed to sync file to disk")?;
    }

    // Rename to target path (atomic operation)
    std::fs::rename(&temp_path, path).with_context(|| {
        format!(
            "Failed to rename {} to {}",
            temp_path.display(),
            path.display()
        )
    })?;

    Ok(())
}

/// Ensure a directory exists, creating it and all parents if needed
pub fn ensure_directory_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    }

    set_permissions_readable_all(path)?;

    Ok(())
}

/// Set file permissions to a specific mode (Unix only, no-op on Windows)
pub fn set_file_permissions(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

        let mut permissions = metadata.permissions();
        permissions.set_mode(mode);

        std::fs::set_permissions(path, permissions)
            .with_context(|| format!("Failed to set permissions for: {}", path.display()))?;
    }

    #[cfg(windows)]
    {
        // On Windows, just ensure it's not read-only
        // More complex ACL manipulation would require additional dependencies
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

        let mut permissions = metadata.permissions();
        permissions.set_readonly(false);

        std::fs::set_permissions(path, permissions)
            .with_context(|| format!("Failed to set permissions for: {}", path.display()))?;
    }

    Ok(())
}

/// Set permissions to make a file or directory readable by all users
pub fn set_permissions_readable_all(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

        let mut permissions = metadata.permissions();

        if path.is_dir() {
            // 755 for directories (rwxr-xr-x)
            permissions.set_mode(0o755);
        } else {
            // 644 for files (rw-r--r--)
            permissions.set_mode(0o644);
        }

        std::fs::set_permissions(path, permissions)
            .with_context(|| format!("Failed to set permissions for: {}", path.display()))?;
    }

    #[cfg(windows)]
    {
        // On Windows, files are typically readable by default
        // More complex ACL manipulation would require additional dependencies
        // For now, we'll just ensure it's not read-only
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

        let mut permissions = metadata.permissions();
        permissions.set_readonly(false);

        std::fs::set_permissions(path, permissions)
            .with_context(|| format!("Failed to set permissions for: {}", path.display()))?;
    }

    Ok(())
}

/// Check if running with administrator/root privileges
pub fn ensure_admin_privileges() -> Result<()> {
    #[cfg(unix)]
    {
        let euid = unsafe { libc::geteuid() };
        if euid != 0 {
            anyhow::bail!(
                "This program must be run as root or with sudo. Current EUID: {}",
                euid
            );
        }
    }

    #[cfg(windows)]
    {
        // On Windows, we check if we can write to a system directory
        // A more robust check would use Windows APIs, but this is a simple approximation
        let test_path = std::path::PathBuf::from(r"C:\Windows\Temp\browser-policy-test.tmp");
        match OpenOptions::new().write(true).create(true).open(&test_path) {
            Ok(_) => {
                // Clean up test file
                let _ = std::fs::remove_file(&test_path);
            }
            Err(_) => {
                anyhow::bail!(
                    "This program must be run as Administrator. Please restart with elevated privileges."
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test_atomic_write.txt");

        let content = b"test content";
        atomic_write(&test_file, content).unwrap();

        let mut file = File::open(&test_file).unwrap();
        let mut read_content = Vec::new();
        file.read_to_end(&mut read_content).unwrap();

        assert_eq!(content, &read_content[..]);

        // temp_dir automatically cleans up when dropped
    }

    #[test]
    fn test_atomic_write_nested_path() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("nested").join("path").join("test.txt");

        let content = b"nested content";
        atomic_write(&test_file, content).unwrap();

        let mut file = File::open(&test_file).unwrap();
        let mut read_content = Vec::new();
        file.read_to_end(&mut read_content).unwrap();

        assert_eq!(content, &read_content[..]);
    }

    #[test]
    fn test_ensure_directory_exists() {
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path().join("test_ensure_dir").join("nested").join("path");

        ensure_directory_exists(&test_dir).unwrap();
        assert!(test_dir.exists());
        assert!(test_dir.is_dir());
    }

    #[test]
    fn test_ensure_directory_exists_idempotent() {
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path().join("idempotent_test");

        // First call creates the directory
        ensure_directory_exists(&test_dir).unwrap();
        assert!(test_dir.exists());

        // Second call should succeed without errors
        ensure_directory_exists(&test_dir).unwrap();
        assert!(test_dir.exists());
    }
}
