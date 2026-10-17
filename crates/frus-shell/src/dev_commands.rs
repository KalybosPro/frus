//! **What the keys of `frus run` ask** a running application (milestone 668), on every
//! platform: the queue the next frame empties, the window a request wakes, and the PNG a
//! screenshot is written as.
//!
//! The commands arrive on another thread — a desktop application's socket ([`crate::dev`]), an
//! Android one's file in its own storage ([`poll_commands`]), a page's `window.__frusCommand`
//! (`web_dev`) — and are carried out by [`crate::app`] at the start of the next frame.

use std::sync::{Arc, Mutex};

use winit::window::Window;

/// What `frus run` asked the application to do, not yet done.
static COMMANDS: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// The window to wake when a request arrives.
static WINDOW: Mutex<Option<Arc<Window>>> = Mutex::new(None);
/// The file, in an Android application's own storage, `frus run` writes its commands to.
#[cfg(android)]
const COMMANDS_FILE: &str = "frus-run-commands";

/// The window a request from `frus run` wakes.
pub(crate) fn set_window(window: Option<Arc<Window>>) {
    if let Ok(mut slot) = WINDOW.lock() {
        *slot = window;
    }
}

/// Asks the window for a frame: a request arrived on another thread, and the frame carries it
/// out. A redraw rather than a bare wake, which Android drops.
pub(crate) fn wake() {
    if let Ok(window) = WINDOW.lock() {
        if let Some(window) = window.as_ref() {
            window.request_redraw();
        }
    }
}

/// Queues a command of `frus run`'s for the next frame, and asks for that frame.
pub(crate) fn push_command(command: &str) {
    let command = command.trim();
    if command.is_empty() {
        return;
    }
    if let Ok(mut queue) = COMMANDS.lock() {
        queue.push(command.to_string());
    }
    wake();
}

/// The commands `frus run` sent since the last frame, oldest first.
pub(crate) fn take_commands() -> Vec<String> {
    COMMANDS
        .lock()
        .map(|mut queue| std::mem::take(&mut *queue))
        .unwrap_or_default()
}

/// Reads the commands `frus run` writes into `storage`, a few times a second, for as long as
/// the application runs: nothing else reaches an Android application from the tool.
#[cfg(android)]
pub(crate) fn poll_commands(storage: std::path::PathBuf) {
    let file = storage.join(COMMANDS_FILE);
    // Commands left by an earlier run are not this one's.
    let _ = std::fs::remove_file(&file);
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let _ = std::fs::remove_file(&file);
        for line in text.lines() {
            push_command(line);
        }
    });
}

/// `rgba` — `width * height * 4` bytes, row by row — as a PNG file: stored, not compressed. A
/// screenshot is a tool's, written once, and the format needs no library this way.
#[cfg_attr(not(desktop), allow(dead_code))]
pub(crate) fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    // 8 bits a channel, RGBA, deflate, adaptive filtering, no interlace.
    header.extend_from_slice(&[8, 6, 0, 0, 0]);

    // The image data: each row behind its filter byte (0, none), in a zlib stream of stored
    // blocks.
    let row = width as usize * 4;
    let mut raw = Vec::with_capacity((row + 1) * height as usize);
    for line in rgba.chunks(row).take(height as usize) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let mut zlib = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65_535).peekable();
    if blocks.peek().is_none() {
        zlib.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    while let Some(block) = blocks.next() {
        zlib.push(u8::from(blocks.peek().is_none()));
        let len = block.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg_attr(not(desktop), allow(dead_code))]
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg_attr(not(desktop), allow(dead_code))]
fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in bytes.chunks(5552) {
        for &byte in chunk {
            a += u32::from(byte);
            b += a;
        }
        a %= 65_521;
        b %= 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The PNG is one any decoder reads: the signature, a checked header, the pixels back.
    #[test]
    fn a_screenshot_is_a_png() {
        let rgba: Vec<u8> = (0..3 * 2)
            .flat_map(|i| [i as u8 * 40, 0, 255 - i as u8, 255])
            .collect();
        let file = png(3, 2, &rgba);
        assert!(file.starts_with(b"\x89PNG\r\n\x1a\n"));
        // The header's CRC, as the format defines it.
        assert_eq!(&file[29..33], &crc32(&file[12..29]).to_be_bytes());
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
        // The pixels are there, row by row behind a zero filter byte, stored.
        let raw: Vec<u8> = rgba
            .chunks(12)
            .flat_map(|row| std::iter::once(0).chain(row.iter().copied()))
            .collect();
        assert!(file.windows(raw.len()).any(|w| w == raw.as_slice()));
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }
}
