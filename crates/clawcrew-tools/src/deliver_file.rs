use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::io::Read;
use std::path::Path;

pub const MAX_DELIVER_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// ACP / model citation URI for an outbound delivered file.
pub fn attachment_deliver_uri(id: &str) -> String {
    format!("attachment://deliver/{id}")
}

/// Open `rel` beneath the already-opened directory handle `dir` (cap-std refuses
/// any component that escapes `dir` via `..` or an escaping symlink), verify it is
/// a regular file within the delivery size cap, and return its bytes.
pub fn read_from_dir(dir: &Dir, rel: &Path) -> Result<Vec<u8>, String> {
    let file = dir
        .open(rel)
        .map_err(|e| format!("Failed to open file: {e}"))?;
    let meta = file
        .metadata()
        .map_err(|e| format!("Failed to read file metadata: {e}"))?;
    if !meta.is_file() {
        return Err("Not a file".to_string());
    }
    if meta.len() > MAX_DELIVER_FILE_BYTES {
        return Err(format!(
            "File too large: {} bytes (limit: {MAX_DELIVER_FILE_BYTES} bytes)",
            meta.len()
        ));
    }
    // One extra byte over the cap catches a grow-after-stat race.
    let mut content = Vec::with_capacity(meta.len() as usize);
    file.take(MAX_DELIVER_FILE_BYTES + 1)
        .read_to_end(&mut content)
        .map_err(|e| format!("Failed to read file: {e}"))?;
    if content.len() as u64 > MAX_DELIVER_FILE_BYTES {
        return Err(format!(
            "File too large: {}+ bytes (limit: {MAX_DELIVER_FILE_BYTES} bytes)",
            content.len()
        ));
    }
    Ok(content)
}

/// Bounded, no-follow read of an already-delivered artifact under the workspace
/// `uploads/` directory, for the ACP consumer that embeds the blob.
pub fn read_delivered_artifact_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "artifact path has no parent directory".to_string())?;
    let name = path
        .file_name()
        .ok_or_else(|| "artifact path has no file name".to_string())?;
    let dir = Dir::open_ambient_dir(parent, ambient_authority())
        .map_err(|e| format!("Failed to open artifact directory: {e}"))?;
    read_from_dir(&dir, Path::new(name))
}
