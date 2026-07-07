//! System-clipboard → page bridge for pasting images and files.
//!
//! WebKitGTK can't hand image/file clipboard data to the page
//! (<https://bugs.webkit.org/show_bug.cgi?id=218519>), so pasting a screenshot
//! or a file copied from the file manager into WhatsApp silently fails. We read
//! the clipboard ourselves and pass the payload back to the page as base64 data
//! URLs, which `clipboard-image.js` turns into a synthetic paste.
//!
//! Two clipboard shapes are handled:
//!   * raw bitmap (a screenshot / "copy image") — via `arboard`;
//!   * one or more files (copied in the file manager) — advertised as a
//!     `text/uri-list` of `file://` URIs, read via `wl-paste`/`xclip`.

use base64::Engine;
use serde::Serialize;

/// One clipboard entry handed to the page.
#[derive(Serialize, Clone)]
pub struct ClipFile {
    /// Suggested file name (for files) or a synthetic one (for a raw bitmap).
    pub name: String,
    /// MIME type, best-effort from the extension (defaults to image/png).
    pub mime: String,
    /// `data:<mime>;base64,<...>` payload.
    pub data_url: String,
}

/// Reads whatever the clipboard holds — a bitmap and/or files — as data URLs.
/// Returns an empty vec when there's nothing pasteable this way (the page then
/// lets the normal paste proceed).
pub fn read_clipboard_files() -> Vec<ClipFile> {
    // Prefer real files (uri-list): copying a picture in the file manager gives
    // both a thumbnail bitmap and the file; the file is the higher-fidelity one.
    let files = read_uri_list_files();
    if !files.is_empty() {
        return files;
    }
    read_bitmap().into_iter().collect()
}

/// Raw bitmap in the clipboard (screenshot / "copy image"), re-encoded to PNG.
fn read_bitmap() -> Option<ClipFile> {
    let mut clipboard = arboard::Clipboard::new().ok()?;
    let image = clipboard.get_image().ok()?;
    let png = encode_png(&image)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);

    Some(ClipFile {
        name: "pasted-image.png".to_string(),
        mime: "image/png".to_string(),
        data_url: format!("data:image/png;base64,{b64}"),
    })
}

/// Files copied from the file manager, advertised as a `text/uri-list`.
fn read_uri_list_files() -> Vec<ClipFile> {
    let Some(raw) = read_clipboard_target("text/uri-list") else {
        return Vec::new();
    };

    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|line| line.strip_prefix("file://"))
        .filter_map(|path| {
            // URI paths are percent-encoded; decode the common `%20` etc.
            let decoded = percent_decode(path);
            let bytes = std::fs::read(&decoded).ok()?;
            // Cap at ~64 MB so a stray huge file can't blow up the IPC payload.
            if bytes.len() > 64 * 1024 * 1024 {
                return None;
            }
            let name = std::path::Path::new(&decoded)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("pasted-file")
                .to_string();
            let mime = mime_from_name(&name);
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Some(ClipFile {
                name,
                mime: mime.clone(),
                data_url: format!("data:{mime};base64,{b64}"),
            })
        })
        .collect()
}

/// Reads a specific clipboard target via wl-paste (Wayland) or xclip (X11).
/// Returns `None` if neither tool is present or the target isn't offered.
fn read_clipboard_target(target: &str) -> Option<String> {
    use std::process::Command;

    // Wayland first (what most current sessions use).
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        if let Ok(out) = Command::new("wl-paste").args(["-t", target, "-n"]).output() {
            if out.status.success() && !out.stdout.is_empty() {
                return Some(String::from_utf8_lossy(&out.stdout).into_owned());
            }
        }
    }

    // X11 fallback.
    if let Ok(out) = Command::new("xclip")
        .args(["-selection", "clipboard", "-t", target, "-o"])
        .output()
    {
        if out.status.success() && !out.stdout.is_empty() {
            return Some(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }

    None
}

/// Minimal percent-decoding for clipboard file URIs (handles `%XX`).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Best-effort MIME from a file extension for the common WhatsApp-attachable
/// types; anything unknown becomes a generic binary the page treats as a file.
fn mime_from_name(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "zip" => "application/zip",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Encodes arboard's RGBA8 image data as PNG bytes.
fn encode_png(image: &arboard::ImageData) -> Option<Vec<u8>> {
    let width = u32::try_from(image.width).ok()?;
    let height = u32::try_from(image.height).ok()?;

    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&image.bytes).ok()?;
    }
    Some(out)
}
