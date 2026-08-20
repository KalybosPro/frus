//! The byte path, end to end over a real socket.
//!
//! The unit tests beside `net.rs` check the builder and the cap; none of them moves a
//! byte. This one does: a one-shot HTTP server on a loopback port, a real `fetch_bytes`
//! against it, and an assertion on the bytes that came back.
//!
//! It matters because the thing milestone 373 changed is exactly the part a builder test
//! cannot see — `into_string()` became `into_reader().take(…)`, and the difference shows
//! up only when something reads a body.

#![cfg(all(feature = "net", not(target_arch = "wasm32")))]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

/// Serves one request, replies with `body` under `status`, and stops.
///
/// Port **0**: the operating system picks a free one, so two tests running at once do
/// not collide and nothing has to guess what is unused on the machine.
fn serve_once(status: &'static str, body: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let addr = listener.local_addr().expect("its address");
    thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        // Read just enough to know the request arrived; the content does not matter.
        let mut scratch = [0u8; 1024];
        let _ = stream.read(&mut scratch);
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
        let _ = stream.flush();
    });
    format!("http://{addr}/")
}

/// Bytes that are **not** text come back intact.
///
/// A PNG starts with `\x89PNG`, whose first byte is not valid UTF-8 on its own — which
/// is the point. Before this milestone the transport called `into_string()` and a body
/// like this could not come back at all.
#[test]
fn bytes_that_are_not_text_survive_the_round_trip() {
    let payload = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0xff, 0x00];
    let url = serve_once("200 OK", payload.clone());
    let got = futures_lite::future::block_on(frus_shell::fetch_bytes(url)).expect("the body");
    assert_eq!(got, payload);
}

/// And text still comes back as text: `send` is `send_bytes` plus the UTF-8 step, so the
/// old path has to keep behaving exactly as it did.
#[test]
fn text_still_arrives_as_text() {
    let url = serve_once("200 OK", b"hello, ada".to_vec());
    let got = futures_lite::future::block_on(frus_shell::fetch(url)).expect("the body");
    assert_eq!(got, "hello, ada");
}

/// A body that is not valid UTF-8 is a `Decode` error through `fetch`, and the same
/// bytes are fine through `fetch_bytes` — the conversion is what fails, not the transfer.
#[test]
fn a_non_text_body_fails_the_text_path_and_not_the_byte_path() {
    let payload = vec![0xff, 0xfe, 0xfd];
    let url = serve_once("200 OK", payload.clone());
    let err = futures_lite::future::block_on(frus_shell::fetch(url)).unwrap_err();
    assert!(matches!(err, frus_shell::FetchError::Decode(_)), "{err}");

    let url = serve_once("200 OK", payload.clone());
    let got = futures_lite::future::block_on(frus_shell::fetch_bytes(url)).expect("the body");
    assert_eq!(got, payload);
}

/// A non-2xx status is a `Status`, whatever the body: the byte path must not swallow an
/// error page and hand it back as content.
#[test]
fn a_failing_status_is_reported_rather_than_returned_as_bytes() {
    let url = serve_once("404 Not Found", b"<html>gone</html>".to_vec());
    let err = futures_lite::future::block_on(frus_shell::fetch_bytes(url)).unwrap_err();
    assert!(matches!(err, frus_shell::FetchError::Status(404)), "{err}");
}
