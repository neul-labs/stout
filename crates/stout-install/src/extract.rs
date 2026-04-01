//! Bottle extraction

use crate::error::{Error, Result};
use flate2::read::GzDecoder;
use memchr::memmem;
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tar::Archive;
use tracing::{debug, info, warn};

/// Extract a bottle tarball to the Cellar
///
/// Bottles are tarballs with structure: `<name>/<version>/...`
/// We extract to: `<cellar>/<name>/<version>/...`
pub fn extract_bottle(bottle_path: impl AsRef<Path>, cellar: impl AsRef<Path>) -> Result<PathBuf> {
    let bottle_path = bottle_path.as_ref();
    let cellar = cellar.as_ref();

    debug!(
        "Extracting {} to {}",
        bottle_path.display(),
        cellar.display()
    );

    let file = File::open(bottle_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    // Create cellar if it doesn't exist
    create_dir_all_force(cellar)?;

    // Extract all entries
    let mut install_path: Option<PathBuf> = None;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        // Get the top-level directory (package name)
        if install_path.is_none() {
            if let Some(component) = path.components().next() {
                let pkg_name = component.as_os_str().to_string_lossy();
                // The path inside the tarball is like `wget/1.24.5/...`
                // We want to extract to `<cellar>/wget/1.24.5/...`
                if let Some(second) = path.components().nth(1) {
                    let version = second.as_os_str().to_string_lossy();
                    let dest_path = cellar.join(&*pkg_name).join(&*version);

                    // Remove existing directory if it exists (from previous failed/partial install)
                    if dest_path.exists() {
                        debug!("Removing existing directory: {}", dest_path.display());
                        std::fs::remove_dir_all(&dest_path)?;
                    }

                    install_path = Some(dest_path);
                }
            }
        }

        // Compute full destination path
        let dest = cellar.join(&path);

        // Create parent directories (removing any conflicting files in the way)
        if let Some(parent) = dest.parent() {
            create_dir_all_force(parent)?;
        }

        // Remove existing file/symlink if present (entry.unpack fails on existing files)
        if dest.exists() || dest.symlink_metadata().is_ok() {
            debug!("Removing existing file: {}", dest.display());
            if dest.is_dir() {
                std::fs::remove_dir_all(&dest)?;
            } else {
                std::fs::remove_file(&dest)?;
            }
        }

        // Extract the entry
        entry.unpack(&dest)?;
    }

    let install_path = install_path.ok_or_else(|| {
        Error::InvalidBottle("Could not determine install path from bottle".to_string())
    })?;

    info!("Extracted to {}", install_path.display());
    Ok(install_path)
}

/// Homebrew placeholder byte patterns used in bottle binaries and text files.
const HOMEBREW_MARKER: &[u8] = b"@@HOMEBREW_";
/// Check if a file is an ELF binary
fn is_elf_binary(path: &Path) -> bool {
    if let Ok(mut file) = File::open(path) {
        let mut magic = [0u8; 4];
        if file.read_exact(&mut magic).is_ok() {
            // ELF magic number: 0x7f 'E' 'L' 'F'
            return magic == [0x7f, b'E', b'L', b'F'];
        }
    }
    false
}

/// Check if a file is a Mach-O binary (macOS executable, dylib, or bundle)
fn is_macho_binary(path: &Path) -> bool {
    if let Ok(mut file) = File::open(path) {
        let mut magic = [0u8; 4];
        if file.read_exact(&mut magic).is_ok() {
            // Mach-O magic numbers (both endianness variants):
            // 0xfeedface / 0xcefaedfe — Mach-O 32-bit
            // 0xfeedfacf / 0xcffaedfe — Mach-O 64-bit
            // 0xcafebabe / 0xbebafeca — FAT / universal binary
            return matches!(
                magic,
                [0xfe, 0xed, 0xfa, 0xce]
                    | [0xce, 0xfa, 0xed, 0xfe]
                    | [0xfe, 0xed, 0xfa, 0xcf]
                    | [0xcf, 0xfa, 0xed, 0xfe]
                    | [0xca, 0xfe, 0xba, 0xbe]
                    | [0xbe, 0xba, 0xfe, 0xca]
            );
        }
    }
    false
}
/// Relocate ELF binary using patchelf
fn relocate_elf_binary(path: &Path, prefix: &str) -> Result<bool> {
    // Check if patchelf is available
    let patchelf = std::process::Command::new("patchelf")
        .arg("--version")
        .output();

    if patchelf.is_err() {
        // patchelf not available, skip with warning
        warn!(
            "patchelf not found - ELF binaries may not work correctly. \
             Install patchelf for proper binary relocation."
        );
        return Ok(false);
    }

    // Read current interpreter
    let output = std::process::Command::new("patchelf")
        .arg("--print-interpreter")
        .arg(path)
        .output();

    let interp = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => return Ok(false), // Not a dynamically linked executable
    };

    // Check if interpreter contains a placeholder
    if !interp.contains("@@HOMEBREW") && !interp.contains("linuxbrew") {
        return Ok(false);
    }

    // Compute new interpreter path
    let new_interp = if interp.contains("@@HOMEBREW_PREFIX@@") {
        interp.replace("@@HOMEBREW_PREFIX@@", prefix)
    } else if interp.contains("/home/linuxbrew/.linuxbrew") {
        interp.replace("/home/linuxbrew/.linuxbrew", prefix)
    } else {
        return Ok(false);
    };

    // Check if the new interpreter exists, if not use system ld
    let final_interp = if Path::new(&new_interp).exists() {
        new_interp
    } else {
        // Fall back to system dynamic linker
        let system_ld = find_system_ld();
        if let Some(ld) = system_ld {
            debug!("Using system linker {} instead of {}", ld, new_interp);
            ld
        } else {
            warn!("Cannot find suitable dynamic linker for {}", path.display());
            return Ok(false);
        }
    };

    // Set the new interpreter
    let result = std::process::Command::new("patchelf")
        .arg("--set-interpreter")
        .arg(&final_interp)
        .arg(path)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            debug!(
                "Patched ELF interpreter: {} -> {}",
                path.display(),
                final_interp
            );
            Ok(true)
        }
        Ok(o) => {
            warn!(
                "patchelf failed for {}: {}",
                path.display(),
                String::from_utf8_lossy(&o.stderr)
            );
            Ok(false)
        }
        Err(e) => {
            warn!("patchelf error for {}: {}", path.display(), e);
            Ok(false)
        }
    }
}

/// Find the system dynamic linker
fn find_system_ld() -> Option<String> {
    let candidates = [
        "/lib64/ld-linux-x86-64.so.2",
        "/lib/ld-linux-x86-64.so.2",
        "/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2",
        "/lib/ld-linux-aarch64.so.1",
        "/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1",
    ];

    for candidate in candidates {
        if Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }
    None
}

/// File extensions that are compressed or non-textual archives — these never
/// contain Homebrew placeholders and can be safely skipped during scanning.
const SCAN_SKIP_EXTENSIONS: &[&str] = &[
    "gz", "bz2", "xz", "zst", "zip", "tar", "png", "jpg", "jpeg", "gif", "ico", "bmp", "tiff",
    "webp", "ttf", "otf", "woff", "woff2", "pyc", "pyo", "class", "jar", "db", "sqlite", "wasm",
];
/// Relocate Homebrew placeholders in the extracted bottle
///
/// Replaces @@HOMEBREW_PREFIX@@ and similar placeholders with actual paths
pub fn relocate_bottle(install_path: impl AsRef<Path>, prefix: impl AsRef<Path>) -> Result<usize> {
    let install_path = install_path.as_ref();
    let prefix = prefix.as_ref();
    let cellar = prefix.join("Cellar");

    let prefix_str = prefix.to_string_lossy();
    let cellar_str = cellar.to_string_lossy();

    // Walk all files, then process in parallel
    let files = walkdir(install_path)?;
    let relocated_count = AtomicUsize::new(0);

    // Clean up orphaned .stout-reloc temp files from interrupted runs
    for path in &files {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".stout-reloc") {
                let _ = fs::remove_file(path);
            }
        }
    }

    files.par_iter().for_each(|path| {
        let metadata = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(_) => return,
        };

        if !metadata.is_file() {
            return;
        }

        if is_elf_binary(path) {
            if relocate_elf_binary(path, &prefix_str).unwrap_or(false) {
                relocated_count.fetch_add(1, Ordering::Relaxed);
            }
        } else if is_macho_binary(path) {
            #[cfg(target_os = "macos")]
            if relocate_macho_binary(path, &prefix_str, &cellar_str).unwrap_or(false) {
                relocated_count.fetch_add(1, Ordering::Relaxed);
            }
        } else if relocate_file(path, &prefix_str, &cellar_str).unwrap_or(false) {
            relocated_count.fetch_add(1, Ordering::Relaxed);
        }
    });

    let count = relocated_count.load(Ordering::Relaxed);
    if count > 0 {
        debug!("Relocated {} files", count);
    }

    Ok(count)
}

/// Recursively walk a directory and return all file paths
fn walkdir(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    walkdir_recursive(dir.as_ref(), &mut files)?;
    Ok(files)
}

fn walkdir_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let ft = fs::symlink_metadata(&path)?.file_type();
            if ft.is_dir() {
                walkdir_recursive(&path, files)?;
            } else if !ft.is_symlink() || !path.is_dir() {
                files.push(path);
            }
        }
    }
    Ok(())
}

/// Relocate a single file, replacing Homebrew placeholders
fn relocate_file(path: &Path, prefix: &str, cellar: &str) -> Result<bool> {
    let _guard = match WriteGuard::acquire(path) {
        Ok(g) => g,
        Err(e) => {
            warn!("Could not make file writable: {}: {}", path.display(), e);
            return Ok(false);
        }
    };

    // Read the file
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            warn!(
                "Could not open file for relocation: {}: {}",
                path.display(),
                e
            );
            return Ok(false);
        }
    };

    let mut contents = Vec::new();
    if let Err(e) = file.read_to_end(&mut contents) {
        warn!(
            "Could not read file for relocation: {}: {}",
            path.display(),
            e
        );
        return Ok(false);
    }
    drop(file);

    // Check if file contains any placeholders
    if memmem::find(&contents, HOMEBREW_MARKER).is_none() {
        return Ok(false);
    }

    // Perform replacements
    let library = format!("{}/Library", prefix);
    let pairs: &[(&[u8], &[u8])] = &[
        (PH_PREFIX, prefix.as_bytes()),
        (PH_CELLAR, cellar.as_bytes()),
        (PH_LIBRARY, library.as_bytes()),
        (PH_REPOSITORY, prefix.as_bytes()),
    ];

    let mut new_contents = contents;
    let mut modified = false;

    for &(needle, replacement) in pairs {
        let after = replace_bytes(&new_contents, needle, replacement);
        if after != new_contents {
            modified = true;
            new_contents = after;
        }
    }

    if modified {
        atomic_write(path, &new_contents)?;
        debug!("Relocated: {}", path.display());
    }

    Ok(modified)
}

/// Replace all occurrences of a byte pattern in a byte vector
///
/// Note: This does NOT pad with nulls, so the file size may change.
/// This works for both text files and binaries where the placeholder
/// is part of a longer path (e.g., @@HOMEBREW_PREFIX@@/lib/ld.so).
fn replace_bytes(haystack: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    if needle.is_empty() {
        return haystack.to_vec();
    }

    let finder = memmem::Finder::new(needle);
    let mut result = Vec::with_capacity(haystack.len());
    let mut start = 0;

    while let Some(pos) = finder.find(&haystack[start..]) {
        result.extend_from_slice(&haystack[start..start + pos]);
        result.extend_from_slice(replacement);
        start += pos + needle.len();
    }

    result.extend_from_slice(&haystack[start..]);
    result
}

fn replace_bytes_padded(haystack: &[u8], needle: &[u8], replacement: &[u8]) -> Option<Vec<u8>> {
    if needle.is_empty() {
        return Some(haystack.to_vec());
    }
    if replacement.len() > needle.len() {
        return None;
    }

    let finder = memmem::Finder::new(needle);
    let mut result = Vec::with_capacity(haystack.len());
    let mut start = 0;

    while let Some(pos) = finder.find(&haystack[start..]) {
        result.extend_from_slice(&haystack[start..start + pos]);
        result.extend_from_slice(replacement);
        // Null-pad to preserve the original needle length
        let pad_len = needle.len() - replacement.len();
        result.extend(std::iter::repeat_n(0, pad_len));
        start += pos + needle.len();
    }

    result.extend_from_slice(&haystack[start..]);
    Some(result)
}
/// Remove an installed package from the Cellar
pub fn remove_package(cellar: impl AsRef<Path>, name: &str, version: &str) -> Result<()> {
    let package_path = cellar.as_ref().join(name).join(version);

    if !package_path.exists() {
        return Err(Error::PackageNotFound(format!("{}/{}", name, version)));
    }

    debug!("Removing {}", package_path.display());
    std::fs::remove_dir_all(&package_path)?;

    // Remove parent directory if empty
    let parent = cellar.as_ref().join(name);
    if parent.read_dir()?.next().is_none() {
        std::fs::remove_dir(&parent)?;
    }

    info!("Removed {}-{}", name, version);
    Ok(())
}

const PH_REPOSITORY: &[u8] = b"@@HOMEBREW_REPOSITORY@@";

/// RAII guard that restores file permissions on drop.
struct WriteGuard<'a> {
    path: &'a Path,
    perms: Option<std::fs::Permissions>,
}

/// Check if a file should be skipped during scanning.
fn should_skip_scan(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SCAN_SKIP_EXTENSIONS.contains(&e))
        .unwrap_or(false)
}

fn parse_macho_load_commands(path: &Path) -> Result<Vec<MachLoadCommand>> {
    let output = std::process::Command::new("otool")
        .arg("-l")
        .arg(path)
        .output()
        .map_err(Error::Io)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commands = Vec::new();

    let mut current_cmd = None;

    for line in stdout.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("cmd ") {
            current_cmd = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("name ") {
            if let Some(cmd) = &current_cmd {
                let name = rest.split('(').next().unwrap_or("").trim().to_string();
                match cmd.as_str() {
                    "LC_ID_DYLIB" => commands.push(MachLoadCommand::DylibId(name)),
                    "LC_LOAD_DYLIB" | "LC_LOAD_WEAK_DYLIB" => {
                        commands.push(MachLoadCommand::LoadDylib(name))
                    }
                    _ => {}
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("path ") {
            if let Some(cmd) = &current_cmd {
                if cmd == "LC_RPATH" {
                    let rpath = rest.split('(').next().unwrap_or("").trim().to_string();
                    commands.push(MachLoadCommand::Rpath(rpath));
                }
            }
        }
    }

    Ok(commands)
}

impl<'a> WriteGuard<'a> {
    /// Make a file writable, returning a guard that restores permissions on drop.
    fn acquire(path: &'a Path) -> Result<Self> {
        let metadata = fs::metadata(path).map_err(Error::Io)?;
        let perms = metadata.permissions();
        let was_readonly = perms.mode() & 0o200 == 0;
        if was_readonly {
            let mut writable = perms.clone();
            writable.set_mode(perms.mode() | 0o200);
            fs::set_permissions(path, writable)?;
        }
        Ok(Self {
            path,
            perms: if was_readonly { Some(perms) } else { None },
        })
    }
}

const PH_PREFIX: &[u8] = b"@@HOMEBREW_PREFIX@@";

const PH_LIBRARY: &[u8] = b"@@HOMEBREW_LIBRARY@@";

impl Drop for WriteGuard<'_> {
    fn drop(&mut self) {
        if let Some(perms) = self.perms.take() {
            let _ = fs::set_permissions(self.path, perms);
        }
    }
}

const PH_CELLAR: &[u8] = b"@@HOMEBREW_CELLAR@@";

enum MachLoadCommand {
    /// LC_ID_DYLIB — the dylib's own identifier
    DylibId(String),
    /// LC_LOAD_DYLIB / LC_LOAD_WEAK_DYLIB — linked library
    LoadDylib(String),
    /// LC_RPATH — runtime search path
    Rpath(String),
}

/// Write data to a file atomically: write to a temp file, then rename.
/// Preserves the original file's permissions.
fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let tmp = path.with_extension(".stout-reloc");

    (|| -> std::io::Result<()> {
        let mut file = File::create(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
        Ok(())
    })()
    .map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        Error::Io(e)
    })?;

    if let Ok(meta) = fs::metadata(path) {
        let _ = fs::set_permissions(&tmp, meta.permissions());
    }

    fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        Error::Io(e)
    })?;

    Ok(())
}

fn replace_homebrew_placeholders(s: &str, prefix: &str, cellar: &str) -> String {
    let library = format!("{}/Library", prefix);
    s.replace("@@HOMEBREW_PREFIX@@", prefix)
        .replace("@@HOMEBREW_CELLAR@@", cellar)
        .replace("@@HOMEBREW_LIBRARY@@", &library)
        .replace("@@HOMEBREW_REPOSITORY@@", prefix)
}

fn relocate_macho_binary(path: &Path, prefix: &str, cellar: &str) -> Result<bool> {
    let _guard = match WriteGuard::acquire(path) {
        Ok(g) => g,
        Err(e) => {
            warn!(
                "Could not make Mach-O binary writable: {}: {}",
                path.display(),
                e
            );
            return Ok(false);
        }
    };

    // Read file and check for any Homebrew placeholders
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            warn!("Could not open Mach-O binary: {}: {}", path.display(), e);
            return Ok(false);
        }
    };
    let mut contents = Vec::new();
    if let Err(e) = file.read_to_end(&mut contents) {
        warn!("Could not read Mach-O binary: {}: {}", path.display(), e);
        return Ok(false);
    }
    drop(file);

    if memmem::find(&contents, HOMEBREW_MARKER).is_none() {
        return Ok(false);
    }

    let mut modified = false;

    // Step 1: Fix load commands using install_name_tool.
    // Must run before byte replacement so otool can parse the original paths.
    match parse_macho_load_commands(path) {
        Ok(load_commands) => {
            for lc in &load_commands {
                let old_path = match lc {
                    MachLoadCommand::DylibId(p)
                    | MachLoadCommand::LoadDylib(p)
                    | MachLoadCommand::Rpath(p) => p.as_str(),
                };

                if !old_path.contains("@@HOMEBREW_") {
                    continue;
                }

                let new_path = replace_homebrew_placeholders(old_path, prefix, cellar);

                let result = match lc {
                    MachLoadCommand::DylibId(_) => std::process::Command::new("install_name_tool")
                        .args(["-id", &new_path])
                        .arg(path)
                        .output(),
                    MachLoadCommand::LoadDylib(_) => {
                        std::process::Command::new("install_name_tool")
                            .args(["-change", old_path, &new_path])
                            .arg(path)
                            .output()
                    }
                    MachLoadCommand::Rpath(_) => {
                        // Rpath needs delete + add (no in-place replace)
                        let del = std::process::Command::new("install_name_tool")
                            .args(["-delete_rpath", old_path])
                            .arg(path)
                            .output();
                        if del.as_ref().is_ok_and(|o| o.status.success()) {
                            std::process::Command::new("install_name_tool")
                                .args(["-add_rpath", &new_path])
                                .arg(path)
                                .output()
                        } else {
                            del
                        }
                    }
                };

                match result {
                    Ok(o) if o.status.success() => {
                        debug!("install_name_tool: {} → {}", old_path, new_path);
                        modified = true;
                    }
                    Ok(o) => {
                        warn!(
                            "install_name_tool failed for {}: {}",
                            path.display(),
                            String::from_utf8_lossy(&o.stderr)
                        );
                    }
                    Err(e) => {
                        warn!("Could not run install_name_tool: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            warn!(
                "Could not parse load commands for {}: {}",
                path.display(),
                e
            );
        }
    }

    // Step 2: Re-read the file if install_name_tool modified it on disk.
    let post_contents = if modified {
        let mut file = File::open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        buf
    } else {
        contents
    };

    // Step 3: Fix remaining embedded strings with null-padded replacement.
    // install_name_tool handles structured load commands, but some binaries
    // (Python, Ruby) embed the prefix in their __TEXT segment for sys.prefix.
    // LC_DYLD_ENVIRONMENT entries are also handled here.
    let library = format!("{}/Library", prefix);
    let pairs: &[(&[u8], &[u8])] = &[
        (PH_PREFIX, prefix.as_bytes()),
        (PH_CELLAR, cellar.as_bytes()),
        (PH_LIBRARY, library.as_bytes()),
        (PH_REPOSITORY, prefix.as_bytes()),
    ];

    let mut new_contents = post_contents;
    let mut embedded_modified = false;

    for &(needle, replacement) in pairs {
        if replacement.len() > needle.len() {
            // Longer replacements (CELLAR, LIBRARY on ARM Mac) are handled by
            // install_name_tool for load commands. For embedded strings, we
            // can't safely expand without parsing Mach-O structure.
            if memmem::find(&new_contents, needle).is_some() {
                debug!(
                    "Skipping longer placeholder {:?} in embedded strings of {} \
                     (expected to be handled by install_name_tool)",
                    std::str::from_utf8(needle).unwrap_or("???"),
                    path.display()
                );
            }
            continue;
        }

        let Some(next) = replace_bytes_padded(&new_contents, needle, replacement) else {
            continue;
        };

        if next != new_contents {
            new_contents = next;
            embedded_modified = true;
        }
    }

    if embedded_modified {
        atomic_write(path, &new_contents)?;
        debug!(
            "Relocated embedded strings in Mach-O binary: {}",
            path.display()
        );
        modified = true;
    }

    // Step 4: Ad-hoc re-sign to fix invalidated code signature.
    // install_name_tool also invalidates signatures, so this covers both steps.
    if modified {
        match std::process::Command::new("codesign")
            .args(["--force", "--sign", "-"])
            .arg(path)
            .output()
        {
            Ok(o) if o.status.success() => {
                debug!("Ad-hoc re-signed: {}", path.display());
            }
            Ok(o) => {
                warn!(
                    "codesign failed for {}: {}",
                    path.display(),
                    String::from_utf8_lossy(&o.stderr)
                );
            }
            Err(e) => {
                warn!("Could not run codesign for {}: {}", path.display(), e);
            }
        }
    }

    Ok(modified)
}

/// Scan a directory for files containing unresolved @@HOMEBREW_*@@ placeholders.
///
/// Scans all files including binaries and dylibs so they can be reported.
/// Both text files and Mach-O binaries are fixed via `relocate_bottle`.
pub fn scan_unrelocated_files(install_path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let install_path = install_path.as_ref();
    let mut unrelocated = Vec::new();

    let finder = memmem::Finder::new(HOMEBREW_MARKER);

    for entry in walkdir(install_path)? {
        let metadata = match fs::symlink_metadata(&entry) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if !metadata.is_file() || should_skip_scan(&entry) {
            continue;
        }

        if let Ok(mut file) = File::open(&entry) {
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() && finder.find(&buf).is_some() {
                unrelocated.push(entry);
            }
        }
    }

    Ok(unrelocated)
}

/// Scan all packages in a cellar directory for unresolved placeholders in parallel.
///
/// Returns a vec of (package_name, package_path, affected_file_count) tuples.
pub fn scan_cellar_unrelocated(
    cellar_packages: &[crate::cellar::CellarPackage],
) -> Vec<(String, PathBuf, usize)> {
    cellar_packages
        .par_iter()
        .filter_map(|pkg| {
            scan_unrelocated_files(&pkg.path)
                .ok()
                .filter(|files| !files.is_empty())
                .map(|files| (pkg.name.clone(), pkg.path.clone(), files.len()))
        })
        .collect()
}

/// Create directory and all parents, removing any conflicting files in the path
///
/// Unlike std::fs::create_dir_all, this will remove files that exist where
/// directories need to be created.
pub(crate) fn create_dir_all_force(path: &Path) -> std::io::Result<()> {
    // First try the normal way
    if path.exists() && path.is_dir() {
        return Ok(());
    }

    // Collect all ancestors that need to be checked
    let mut to_create: Vec<&Path> = Vec::new();
    let mut current = path;

    // Find the first ancestor that exists
    while !current.exists() {
        to_create.push(current);
        match current.parent() {
            Some(p) if !p.as_os_str().is_empty() => current = p,
            _ => break,
        }
    }
    to_create.reverse();

    // Check each ancestor - if any is a file, remove it
    for dir_path in &to_create {
        if dir_path.symlink_metadata().is_ok() && !dir_path.is_dir() {
            debug!(
                "Removing conflicting file at {}: need directory",
                dir_path.display()
            );
            std::fs::remove_file(dir_path)?;
        }
    }

    // Now create directories
    std::fs::create_dir_all(path)
}
