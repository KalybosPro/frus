//! A WebSocket client, the little `frus run --hot -d web` needs (milestone 679): it connects to
//! the hot-reload engine's socket and reads the text messages it sends, each patch among them.
//! Nothing is sent but the handshake and the answers to pings.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

/// Connects to `address` (`ws://host:port/path`) and hands each text message to `each`, until
/// the server closes the connection or it fails.
pub(crate) fn read_messages(address: &str, mut each: impl FnMut(String)) -> std::io::Result<()> {
    let invalid =
        |why: &str| std::io::Error::new(std::io::ErrorKind::InvalidInput, why.to_string());
    let rest = address
        .strip_prefix("ws://")
        .ok_or_else(|| invalid("not a ws:// address"))?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let mut stream = TcpStream::connect(host)?;
    // The key is the specification's sample: the server's answer to it is not checked.
    write!(
        stream,
        "GET /{path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    )?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut status = String::new();
    reader.read_line(&mut status)?;
    if !status.contains(" 101 ") {
        return Err(invalid(&format!("the server refused: {}", status.trim())));
    }
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim().is_empty() {
            break;
        }
    }
    let mut message = Vec::new();
    loop {
        let mut head = [0u8; 2];
        reader.read_exact(&mut head)?;
        let (fin, opcode) = (head[0] & 0x80 != 0, head[0] & 0x0f);
        let masked = head[1] & 0x80 != 0;
        let length = match head[1] & 0x7f {
            126 => {
                let mut bytes = [0u8; 2];
                reader.read_exact(&mut bytes)?;
                u16::from_be_bytes(bytes) as u64
            }
            127 => {
                let mut bytes = [0u8; 8];
                reader.read_exact(&mut bytes)?;
                u64::from_be_bytes(bytes)
            }
            n => n as u64,
        };
        let mut mask = [0u8; 4];
        if masked {
            reader.read_exact(&mut mask)?;
        }
        let mut payload = vec![0u8; length as usize];
        reader.read_exact(&mut payload)?;
        if masked {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[i % 4];
            }
        }
        match opcode {
            // Text, binary, and the continuation of either.
            0..=2 => {
                message.extend_from_slice(&payload);
                if fin {
                    if opcode != 2 {
                        each(String::from_utf8_lossy(&message).into_owned());
                    }
                    message.clear();
                }
            }
            // Close.
            8 => return Ok(()),
            // Ping: a pong with the same payload. A client's frames are masked; a zero mask
            // leaves the payload as it is.
            9 => {
                let mut pong = vec![0x8a];
                if payload.len() < 126 {
                    pong.push(0x80 | payload.len() as u8);
                } else {
                    pong.push(0x80 | 126);
                    pong.extend_from_slice(&(payload.len() as u16).to_be_bytes());
                }
                pong.extend_from_slice(&[0, 0, 0, 0]);
                pong.extend_from_slice(&payload);
                stream.write_all(&pong)?;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn frame(fin: bool, opcode: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![if fin { 0x80 } else { 0 } | opcode];
        match payload.len() {
            n if n < 126 => out.push(n as u8),
            n if n < 65536 => {
                out.push(126);
                out.extend_from_slice(&(n as u16).to_be_bytes());
            }
            n => {
                out.push(127);
                out.extend_from_slice(&(n as u64).to_be_bytes());
            }
        }
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn the_messages_are_read_whole() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let long = "x".repeat(70_000);
        let middle = "y".repeat(300);
        let (long_sent, middle_sent) = (long.clone(), middle.clone());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                request.push_str(&line);
                if line.trim().is_empty() {
                    break;
                }
            }
            stream
                .write_all(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\r\n")
                .unwrap();
            stream.write_all(&frame(true, 1, b"first")).unwrap();
            stream.write_all(&frame(true, 9, b"are you there")).unwrap();
            stream.write_all(&frame(false, 1, b"in ")).unwrap();
            stream.write_all(&frame(true, 0, b"two parts")).unwrap();
            stream
                .write_all(&frame(true, 1, middle_sent.as_bytes()))
                .unwrap();
            stream
                .write_all(&frame(true, 1, long_sent.as_bytes()))
                .unwrap();
            stream.write_all(&frame(true, 8, b"")).unwrap();
            // The pong the client sent back to the ping.
            let mut pong = [0u8; 2 + 4 + 13];
            reader.read_exact(&mut pong).unwrap();
            (request, pong)
        });
        let mut read = Vec::new();
        read_messages(&format!("ws://127.0.0.1:{port}/_dioxus?build_id=0"), |m| {
            read.push(m)
        })
        .unwrap();
        let (request, pong) = server.join().unwrap();
        assert!(
            request.starts_with("GET /_dioxus?build_id=0 HTTP/1.1\r\n"),
            "{request}"
        );
        assert!(request.contains("Upgrade: websocket"), "{request}");
        assert_eq!(
            read,
            ["first".to_string(), "in two parts".into(), middle, long]
        );
        assert_eq!(&pong[..2], &[0x8a, 0x80 | 13]);
        assert_eq!(&pong[6..], b"are you there");
    }

    #[test]
    fn a_refusal_is_an_error() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut line)
                .unwrap();
            stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
        });
        assert!(read_messages(&format!("ws://127.0.0.1:{port}/"), |_| {}).is_err());
        assert!(read_messages("http://127.0.0.1:1/", |_| {}).is_err());
    }
}
