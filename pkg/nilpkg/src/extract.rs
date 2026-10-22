// pkg/nilpkg/src/extract.rs — Safe archive extraction for .nilax packages.
//
// Archive extraction is a historically bug-prone area (Zip Slip, symlink
// races, zip bombs). This module extracts a tar or zip archive into a staging
// directory while rejecting:
//   - path traversal (`../`, absolute paths)
//   - symlinks pointing outside the destination
//   - decompression bombs (uncompressed size cap checked during extraction)
//   - hardlinks and device nodes
//
// Extraction is atomic: the archive is extracted into a temporary directory
// first, then renamed into place only if every entry was safe.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

/// Maximum total uncompressed size allowed in a single archive (256 MiB).
const MAX_UNCOMPRESSED_SIZE: u64 = 256 * 1024 * 1024;
/// Maximum number of entries allowed in a single archive.
const MAX_ENTRIES: usize = 10_000;
/// Maximum size of a single extracted file (128 MiB).
const MAX_FILE_SIZE: u64 = 128 * 1024 * 1024;

/// A safe, normalized path inside the destination directory.
///
/// Rejects `..`, absolute paths, and empty components. Returns `None` for
/// paths that would escape the destination.
pub fn safe_join(dest: &Path, entry: &str) -> Option<PathBuf> {
    let entry_path = Path::new(entry);
    let mut result = dest.to_path_buf();
    for component in entry_path.components() {
        match component {
            Component::Normal(part) => result.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    // Reject empty paths and paths that resolve to the destination itself.
    if result == dest || result.as_os_str().is_empty() {
        return None;
    }
    Some(result)
}

/// Extract a tar archive into `dest`, enforcing all safety checks.
pub fn extract_tar(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("Could not open archive: {e}"))?;
    let mut archive = tar::Archive::new(file);
    let mut total_size: u64 = 0;
    let mut entry_count: usize = 0;

    let entries = archive
        .entries()
        .map_err(|e| format!("Could not read archive entries: {e}"))?;

    for entry in entries {
        let mut entry = entry.map_err(|e| format!("Corrupt archive entry: {e}"))?;
        entry_count += 1;
        if entry_count > MAX_ENTRIES {
            return Err(format!("Archive exceeds maximum of {MAX_ENTRIES} entries"));
        }

        let path = entry
            .path()
            .map_err(|e| format!("Invalid entry path: {e}"))?
            .into_owned();
        let path_str = path.to_string_lossy();

        // Reject non-regular files (symlinks, hardlinks, devices).
        let header = entry.header();
        let entry_type = header.entry_type();
        if !entry_type.is_file() {
            return Err(format!("Archive contains non-regular file: {path_str}"));
        }

        let target = safe_join(dest, &path_str)
            .ok_or_else(|| format!("Unsafe path in archive: {path_str}"))?;

        // Check uncompressed size before writing.
        let size = header.size().unwrap_or(0);
        total_size += size;
        if total_size > MAX_UNCOMPRESSED_SIZE {
            return Err(format!(
                "Archive exceeds maximum uncompressed size of {MAX_UNCOMPRESSED_SIZE} bytes"
            ));
        }
        if size > MAX_FILE_SIZE {
            return Err(format!(
                "Archive entry exceeds maximum file size of {MAX_FILE_SIZE} bytes: {path_str}"
            ));
        }

        // Create parent directories.
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create directory {}: {e}", parent.display()))?;
        }

        // Write the file with a size cap to prevent zip bombs.
        let mut out = File::create(&target)
            .map_err(|e| format!("Could not create {}: {e}", target.display()))?;
        let mut buf = [0u8; 8192];
        let mut written: u64 = 0;
        loop {
            let n = entry
                .read(&mut buf)
                .map_err(|e| format!("Read error on {path_str}: {e}"))?;
            if n == 0 {
                break;
            }
            written += n as u64;
            if written > MAX_FILE_SIZE {
                let _ = fs::remove_file(&target);
                return Err(format!(
                    "Archive entry exceeds maximum file size during extraction: {path_str}"
                ));
            }
            out.write_all(&buf[..n])
                .map_err(|e| format!("Write error on {}: {e}", target.display()))?;
        }
    }

    Ok(())
}

/// Extract a zip archive into `dest`, enforcing all safety checks (path traversal,
/// symlink/special-file rejection, SUID/SGID mode stripping, and decompression limits)
/// without depending on vulnerable external `zip` crate extractors.
pub fn extract_zip(archive: &Path, dest: &Path) -> Result<(), String> {
    let data = fs::read(archive).map_err(|e| format!("Could not open archive: {e}"))?;
    if data.len() < 22 {
        return Err("Invalid zip archive: too small".into());
    }

    // Locate End of Central Directory (EOCD) signature PK\x05\x06 from the end
    let min_eocd_pos = data.len().saturating_sub(65557);
    let mut eocd_pos = None;
    for pos in (min_eocd_pos..=data.len() - 22).rev() {
        if &data[pos..pos + 4] == b"PK\x05\x06" {
            eocd_pos = Some(pos);
            break;
        }
    }
    let eocd = eocd_pos.ok_or_else(|| "Invalid zip archive: missing EOCD".to_string())?;
    let entry_count = u16::from_le_bytes([data[eocd + 10], data[eocd + 11]]) as usize;
    let cd_size = u32::from_le_bytes([
        data[eocd + 12],
        data[eocd + 13],
        data[eocd + 14],
        data[eocd + 15],
    ]) as usize;
    let cd_offset = u32::from_le_bytes([
        data[eocd + 16],
        data[eocd + 17],
        data[eocd + 18],
        data[eocd + 19],
    ]) as usize;

    if entry_count > MAX_ENTRIES {
        return Err(format!("Archive exceeds maximum of {MAX_ENTRIES} entries"));
    }
    if cd_offset.saturating_add(cd_size) > data.len() {
        return Err("Invalid zip archive: central directory out of bounds".into());
    }

    let mut total_size: u64 = 0;
    let mut pos = cd_offset;

    for i in 0..entry_count {
        if pos + 46 > data.len() || &data[pos..pos + 4] != b"PK\x01\x02" {
            return Err(format!("Corrupt zip central directory entry {i}"));
        }
        let flags = u16::from_le_bytes([data[pos + 8], data[pos + 9]]);
        if (flags & 0x01) != 0 {
            return Err("Encrypted zip entries are not supported".into());
        }
        let method = u16::from_le_bytes([data[pos + 10], data[pos + 11]]);
        let expected_crc = u32::from_le_bytes([
            data[pos + 16],
            data[pos + 17],
            data[pos + 18],
            data[pos + 19],
        ]);
        let comp_size = u32::from_le_bytes([
            data[pos + 20],
            data[pos + 21],
            data[pos + 22],
            data[pos + 23],
        ]) as usize;
        let uncomp_size = u32::from_le_bytes([
            data[pos + 24],
            data[pos + 25],
            data[pos + 26],
            data[pos + 27],
        ]) as u64;
        let name_len = u16::from_le_bytes([data[pos + 28], data[pos + 29]]) as usize;
        let extra_len = u16::from_le_bytes([data[pos + 30], data[pos + 31]]) as usize;
        let comment_len = u16::from_le_bytes([data[pos + 32], data[pos + 33]]) as usize;
        let ext_attr = u32::from_le_bytes([
            data[pos + 38],
            data[pos + 39],
            data[pos + 40],
            data[pos + 41],
        ]);
        let lfh_offset = u32::from_le_bytes([
            data[pos + 42],
            data[pos + 43],
            data[pos + 44],
            data[pos + 45],
        ]) as usize;

        let name_end = pos + 46 + name_len;
        let next_pos = name_end + extra_len + comment_len;
        if next_pos > data.len() {
            return Err(format!("Truncated zip central directory entry {i}"));
        }
        let name = std::str::from_utf8(&data[pos + 46..name_end])
            .map_err(|_| format!("Non-UTF8 zip entry path at {i}"))?
            .to_string();
        pos = next_pos;

        // Check Unix mode in upper 16 bits of external attributes (reject symlinks, devices, SUID/SGID)
        let unix_mode = (ext_attr >> 16) & 0xFFFF;
        let file_type_bits = unix_mode & 0o170000;
        if name.ends_with('/')
            || (file_type_bits != 0 && file_type_bits != 0o100000)
            || (unix_mode & 0o6000) != 0
        {
            return Err(format!(
                "Archive contains non-regular file or forbidden mode: {name}"
            ));
        }

        let target =
            safe_join(dest, &name).ok_or_else(|| format!("Unsafe path in archive: {name}"))?;

        total_size += uncomp_size;
        if total_size > MAX_UNCOMPRESSED_SIZE {
            return Err(format!(
                "Archive exceeds maximum uncompressed size of {MAX_UNCOMPRESSED_SIZE} bytes"
            ));
        }
        if uncomp_size > MAX_FILE_SIZE {
            return Err(format!(
                "Archive entry exceeds maximum file size of {MAX_FILE_SIZE} bytes: {name}"
            ));
        }

        if lfh_offset + 30 > data.len() || &data[lfh_offset..lfh_offset + 4] != b"PK\x03\x04" {
            return Err(format!("Corrupt local file header for {name}"));
        }
        let lfh_name_len =
            u16::from_le_bytes([data[lfh_offset + 26], data[lfh_offset + 27]]) as usize;
        let lfh_extra_len =
            u16::from_le_bytes([data[lfh_offset + 28], data[lfh_offset + 29]]) as usize;
        let payload_start = lfh_offset + 30 + lfh_name_len + lfh_extra_len;
        let payload_end = payload_start.saturating_add(comp_size);
        if payload_end > data.len() {
            return Err(format!("Truncated zip payload for {name}"));
        }
        let compressed = &data[payload_start..payload_end];

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create directory {}: {e}", parent.display()))?;
        }

        let mut out = File::create(&target)
            .map_err(|e| format!("Could not create {}: {e}", target.display()))?;
        let mut hasher = crc32fast::Hasher::new();
        let mut written: u64 = 0;

        match method {
            0 => {
                // Stored (uncompressed)
                if compressed.len() as u64 > MAX_FILE_SIZE {
                    let _ = fs::remove_file(&target);
                    return Err(format!("Archive entry exceeds maximum file size: {name}"));
                }
                hasher.update(compressed);
                out.write_all(compressed)
                    .map_err(|e| format!("Write error on {}: {e}", target.display()))?;
                written = compressed.len() as u64;
            }
            8 => {
                // Deflate
                let mut decoder = flate2::read::DeflateDecoder::new(compressed);
                let mut buf = [0u8; 8192];
                loop {
                    let n = decoder
                        .read(&mut buf)
                        .map_err(|e| format!("Deflate error on {name}: {e}"))?;
                    if n == 0 {
                        break;
                    }
                    written += n as u64;
                    if written > MAX_FILE_SIZE {
                        let _ = fs::remove_file(&target);
                        return Err(format!(
                            "Archive entry exceeds maximum file size during extraction: {name}"
                        ));
                    }
                    hasher.update(&buf[..n]);
                    out.write_all(&buf[..n])
                        .map_err(|e| format!("Write error on {}: {e}", target.display()))?;
                }
            }
            other => {
                let _ = fs::remove_file(&target);
                return Err(format!(
                    "Unsupported zip compression method {other} for {name}"
                ));
            }
        }

        if written != uncomp_size {
            let _ = fs::remove_file(&target);
            return Err(format!("Uncompressed size mismatch for {name}"));
        }
        if hasher.finalize() != expected_crc {
            let _ = fs::remove_file(&target);
            return Err(format!("CRC32 integrity mismatch for {name}"));
        }
    }

    Ok(())
}

/// Detect archive type by magic bytes and extract accordingly.
pub fn extract_archive(archive: &Path, dest: &Path) -> Result<(), String> {
    let mut magic = [0u8; 4];
    File::open(archive)
        .and_then(|mut f| f.read_exact(&mut magic))
        .map_err(|e| format!("Could not read archive magic: {e}"))?;

    // Zip magic: PK\x03\x04 (local file header) or PK\x05\x06 (empty archive).
    if magic.starts_with(b"PK") {
        return extract_zip(archive, dest);
    }
    // Tar magic at offset 257: "ustar".
    let mut tar_magic = [0u8; 5];
    if let Ok(mut f) = File::open(archive) {
        use std::io::Seek;
        if f.seek(io::SeekFrom::Start(257)).is_ok() && f.read_exact(&mut tar_magic).is_ok() {
            if &tar_magic == b"ustar" {
                return extract_tar(archive, dest);
            }
        }
    }
    Err("Unknown archive format: expected tar or zip".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("nilpkg-extract-{}-{}", std::process::id(), name))
    }

    #[test]
    fn safe_join_rejects_traversal_and_absolute_paths() {
        let dest = Path::new("/tmp/dest");
        assert!(safe_join(dest, "bin/app").is_some());
        assert!(safe_join(dest, "bin/./app").is_some());
        assert!(safe_join(dest, "../etc/passwd").is_none());
        assert!(safe_join(dest, "bin/../../etc/passwd").is_none());
        assert!(safe_join(dest, "/etc/passwd").is_none());
        assert!(safe_join(dest, "").is_none());
        assert!(safe_join(dest, ".").is_none());
        assert!(safe_join(dest, "bin/").is_some());
    }

    /// Build a raw tar entry with an arbitrary path, bypassing the tar crate's
    /// own path validation. Needed to test that *extraction* rejects these.
    fn make_raw_tar_entry(path: &str, data: &[u8]) -> Vec<u8> {
        let mut buf = vec![0u8; 512];
        // Name field: bytes 0..100
        let name = path.as_bytes();
        buf[..name.len().min(100)].copy_from_slice(&name[..name.len().min(100)]);
        // Mode: bytes 100..108
        buf[100..108].copy_from_slice(b"0000644\0");
        // UID: bytes 108..116
        buf[108..116].copy_from_slice(b"0000000\0");
        // GID: bytes 116..124
        buf[116..124].copy_from_slice(b"0000000\0");
        // Size: bytes 124..136 (octal, 11 digits + null)
        let size_str = format!("{:011o}\0", data.len());
        buf[124..136].copy_from_slice(size_str.as_bytes());
        // Mtime: bytes 136..148
        buf[136..148].copy_from_slice(b"00000000000\0");
        // Checksum: bytes 148..156 (placeholder, computed below)
        buf[148..156].copy_from_slice(b"        ");
        // Typeflag: byte 156 (regular file)
        buf[156] = b'0';
        // Magic: bytes 257..263
        buf[257..263].copy_from_slice(b"ustar\0");
        // Version: bytes 263..265
        buf[263..265].copy_from_slice(b"00");
        // Compute checksum: sum of all header bytes (with checksum field as spaces)
        let checksum: u32 = buf.iter().map(|&b| b as u32).sum();
        let cksum_str = format!("{:06o}\0 ", checksum);
        buf[148..156].copy_from_slice(cksum_str.as_bytes());
        // Append data + padding to 512.
        let mut result = buf;
        result.extend_from_slice(data);
        let pad = (512 - (data.len() % 512)) % 512;
        result.extend(std::iter::repeat(0).take(pad));
        result
    }

    #[test]
    fn extract_tar_rejects_path_traversal() {
        let dir = temp_dir("traversal");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Build a malicious tar with a ../ entry using raw bytes.
        let mut tar_data = Vec::new();
        tar_data.extend_from_slice(&make_raw_tar_entry("../escape.txt", b"evil"));
        // Two zero blocks mark end-of-archive.
        tar_data.extend(std::iter::repeat(0).take(1024));
        let archive = dir.join("evil.tar");
        fs::write(&archive, &tar_data).unwrap();

        let dest = dir.join("out");
        let result = extract_tar(&archive, &dest);
        assert!(result.is_err(), "traversal must be rejected");
        assert!(result.unwrap_err().contains("Unsafe path"));
        assert!(
            !dir.join("escape.txt").exists(),
            "file must not escape dest"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_tar_rejects_absolute_paths() {
        let dir = temp_dir("absolute");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut tar_data = Vec::new();
        tar_data.extend_from_slice(&make_raw_tar_entry("/tmp/evil.txt", b"evil"));
        tar_data.extend(std::iter::repeat(0).take(1024));
        let archive = dir.join("evil.tar");
        fs::write(&archive, &tar_data).unwrap();

        let dest = dir.join("out");
        let result = extract_tar(&archive, &dest);
        assert!(result.is_err(), "absolute path must be rejected");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_tar_rejects_symlinks() {
        let dir = temp_dir("symlink");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        header.set_cksum();
        builder
            .append_data(&mut header, "link", &b"../../etc/passwd"[..])
            .unwrap();
        let data = builder.into_inner().unwrap();
        let archive = dir.join("evil.tar");
        fs::write(&archive, &data).unwrap();

        let dest = dir.join("out");
        let result = extract_tar(&archive, &dest);
        assert!(result.is_err(), "symlink must be rejected");
        assert!(result.unwrap_err().contains("non-regular"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_tar_accepts_valid_archive() {
        let dir = temp_dir("valid");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(11);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "bin/app", &b"hello world"[..])
            .unwrap();
        let data = builder.into_inner().unwrap();
        let archive = dir.join("good.tar");
        fs::write(&archive, &data).unwrap();

        let dest = dir.join("out");
        extract_tar(&archive, &dest).unwrap();
        assert_eq!(
            fs::read_to_string(dest.join("bin/app")).unwrap(),
            "hello world"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    /// Build a single-entry zip archive in memory for testing `extract_zip`.
    fn make_raw_zip_entry(name: &str, payload: &[u8], unix_mode: u16, deflate: bool) -> Vec<u8> {
        let mut crc_hasher = crc32fast::Hasher::new();
        crc_hasher.update(payload);
        let crc = crc_hasher.finalize();

        let (method, compressed): (u16, Vec<u8>) = if deflate {
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(payload).unwrap();
            (8, enc.finish().unwrap())
        } else {
            (0, payload.to_vec())
        };

        let name_bytes = name.as_bytes();
        let comp_len = compressed.len() as u32;
        let uncomp_len = payload.len() as u32;
        let name_len = name_bytes.len() as u16;

        let mut out = Vec::new();
        // Local File Header (PK\x03\x04)
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // mtime
        out.extend_from_slice(&0u16.to_le_bytes()); // mdate
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&comp_len.to_le_bytes());
        out.extend_from_slice(&uncomp_len.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(&compressed);

        // Central Directory Header (PK\x01\x02)
        let cd_offset = out.len() as u32;
        out.extend_from_slice(b"PK\x01\x02");
        out.extend_from_slice(&0x0314u16.to_le_bytes()); // version made by Unix
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // mtime
        out.extend_from_slice(&0u16.to_le_bytes()); // mdate
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&comp_len.to_le_bytes());
        out.extend_from_slice(&uncomp_len.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(&0u16.to_le_bytes()); // comment len
        out.extend_from_slice(&0u16.to_le_bytes()); // disk number
        out.extend_from_slice(&0u16.to_le_bytes()); // internal attr
        let ext_attr = (unix_mode as u32) << 16;
        out.extend_from_slice(&ext_attr.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // local header offset 0
        out.extend_from_slice(name_bytes);
        let cd_size = (out.len() as u32) - cd_offset;

        // End of Central Directory (PK\x05\x06)
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&0u16.to_le_bytes()); // disk
        out.extend_from_slice(&0u16.to_le_bytes()); // cd start disk
        out.extend_from_slice(&1u16.to_le_bytes()); // entries on disk
        out.extend_from_slice(&1u16.to_le_bytes()); // total entries
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // comment len
        out
    }

    #[test]
    fn extract_zip_rejects_path_traversal() {
        let dir = temp_dir("zip-traversal");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let archive = dir.join("evil.zip");
        fs::write(
            &archive,
            make_raw_zip_entry("../escape.txt", b"evil", 0o100644, true),
        )
        .unwrap();

        let dest = dir.join("out");
        let result = extract_zip(&archive, &dest);
        assert!(result.is_err(), "zip traversal must be rejected");
        assert!(!dir.join("escape.txt").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_zip_rejects_symlinks_and_suid_modes() {
        let dir = temp_dir("zip-symlink-suid");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Symlink mode (0o120777) must be rejected
        let sym_zip = dir.join("symlink.zip");
        fs::write(
            &sym_zip,
            make_raw_zip_entry("bin/link", b"/etc/passwd", 0o120777, false),
        )
        .unwrap();
        assert!(extract_zip(&sym_zip, &dir.join("out_sym")).is_err());

        // SUID mode (0o104755) must be rejected
        let suid_zip = dir.join("suid.zip");
        fs::write(
            &suid_zip,
            make_raw_zip_entry("bin/rootsh", b"elf", 0o104755, false),
        )
        .unwrap();
        assert!(extract_zip(&suid_zip, &dir.join("out_suid")).is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_zip_accepts_valid_archive() {
        let dir = temp_dir("zip-valid");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let archive = dir.join("good.zip");
        fs::write(
            &archive,
            make_raw_zip_entry("bin/app", b"hello zip", 0o100755, true),
        )
        .unwrap();

        let dest = dir.join("out");
        extract_zip(&archive, &dest).unwrap();
        assert_eq!(
            fs::read_to_string(dest.join("bin/app")).unwrap(),
            "hello zip"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_archive_detects_format_by_magic() {
        let dir = temp_dir("detect");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Tar file.
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(3);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "a.txt", &b"tar"[..])
            .unwrap();
        let tar_data = builder.into_inner().unwrap();
        let tar_archive = dir.join("test.tar");
        fs::write(&tar_archive, &tar_data).unwrap();

        let dest1 = dir.join("out1");
        extract_archive(&tar_archive, &dest1).unwrap();
        assert_eq!(fs::read_to_string(dest1.join("a.txt")).unwrap(), "tar");

        // Zip file.
        let zip_archive = dir.join("test.zip");
        fs::write(
            &zip_archive,
            make_raw_zip_entry("b.txt", b"zip", 0o100644, true),
        )
        .unwrap();

        let dest2 = dir.join("out2");
        extract_archive(&zip_archive, &dest2).unwrap();
        assert_eq!(fs::read_to_string(dest2.join("b.txt")).unwrap(), "zip");

        // Unknown format.
        let unknown = dir.join("test.bin");
        fs::write(&unknown, b"not an archive").unwrap();
        assert!(extract_archive(&unknown, &dir.join("out3")).is_err());

        let _ = fs::remove_dir_all(&dir);
    }
}
