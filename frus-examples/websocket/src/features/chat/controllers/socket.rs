//! The socket thread: everything that blocks on the network lives here.
//!
//! It owns one `tungstenite` socket from the first byte to the last, so no lock is
//! needed. One loop does both directions: send what the page queued, then wait a short
//! while for the server ([`POLL`]); a read that times out is not an error, just a turn of
//! the loop. That short wait is also how quickly a message typed in the page leaves.

use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;

use tungstenite::client::IntoClientRequest;
use tungstenite::protocol::CloseFrame;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{HandshakeError, Message, WebSocket};

use super::connection::{Event, Outgoing};

pub type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

/// How long to try reaching the server before giving up.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long the TLS and WebSocket handshakes may take.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// How long one read waits for the server before the loop looks at the outgoing queue.
pub const POLL: Duration = Duration::from_millis(50);

/// How many polls to wait for the server to acknowledge our close (≈ 2 s).
pub const CLOSE_POLLS: u32 = 40;

/// Connects to `url`, then relays both ways until the page closes or the network fails.
/// Every outcome is reported on `events`; the thread ends right after `Closed`/`Failed`.
pub fn run(url: &str, requests: Receiver<Outgoing>, events: Sender<Event>) {
    let mut socket = match connect(url) {
        Ok(socket) => socket,
        Err(error) => {
            frus::log::warn!("websocket: could not connect to {url}: {error}");
            let _ = events.send(Event::Failed(error));
            return;
        }
    };
    let _ = events.send(Event::Connected);
    // Set when the server starts closing, reported once the close is complete.
    let mut close_reason = None;

    loop {
        // 1. What the page wants sent, all of it, without waiting.
        loop {
            match requests.try_recv() {
                Ok(Outgoing::Text(text)) => {
                    if let Err(error) = socket.send(Message::text(text)) {
                        fail(&events, url, error.to_string());
                        return;
                    }
                }
                // `Disconnected`: the page dropped its `Connection` — it went away.
                Ok(Outgoing::Close) | Err(TryRecvError::Disconnected) => {
                    close(&mut socket);
                    let _ = events.send(Event::Closed(None));
                    return;
                }
                Err(TryRecvError::Empty) => break,
            }
        }

        // 2. What the server says, waiting at most `POLL`.
        match socket.read() {
            Ok(Message::Text(text)) => {
                if events.send(Event::Received(text.to_string())).is_err() {
                    // Nobody is listening any more: the page is gone.
                    close(&mut socket);
                    return;
                }
            }
            Ok(Message::Binary(bytes)) => {
                let _ = events.send(Event::Received(format!(
                    "[{} bytes of binary]",
                    bytes.len()
                )));
            }
            // The server is closing. tungstenite answers the close frame for us; a later
            // read reports `ConnectionClosed`, below, once both sides are done.
            Ok(Message::Close(frame)) => close_reason = frame.map(describe_close),
            // Pings are answered by tungstenite itself; pongs and raw frames need nothing.
            Ok(_) => {}
            Err(tungstenite::Error::Io(error)) if is_timeout(error.kind()) => {}
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                let _ = events.send(Event::Closed(close_reason));
                return;
            }
            Err(error) => {
                fail(&events, url, error.to_string());
                return;
            }
        }
    }
}

pub fn fail(events: &Sender<Event>, url: &str, error: String) {
    frus::log::warn!("websocket: connection to {url} lost: {error}");
    let _ = events.send(Event::Failed(error));
}

/// TCP with a time limit, then the TLS handshake (for `wss://`) and the WebSocket one.
///
/// `tungstenite::connect(url)` does all of this in one call, but waits for the operating
/// system's own connect timeout, which can be minutes. Opening the TCP stream ourselves
/// lets the page say "unreachable" after [`CONNECT_TIMEOUT`].
pub fn connect(url: &str) -> Result<Socket, String> {
    let request = url
        .into_client_request()
        .map_err(|error| format!("Not a WebSocket address: {error}"))?;
    let uri = request.uri();
    let default_port = match uri.scheme_str() {
        Some("wss") => 443,
        Some("ws") => 80,
        _ => return Err("The address must start with ws:// or wss://".into()),
    };
    let host = uri
        .host()
        .ok_or_else(|| "The address has no host name".to_string())?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_owned();
    let port = uri.port_u16().unwrap_or(default_port);

    let addresses = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|error| format!("Could not find {host}: {error}"))?;
    let stream = addresses
        .into_iter()
        .find_map(|address| TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).ok())
        .ok_or_else(|| format!("Could not reach {host}:{port}"))?;
    // Small messages go out at once rather than waiting to be batched.
    stream
        .set_nodelay(true)
        .map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(HANDSHAKE_TIMEOUT))
        .map_err(|error| error.to_string())?;

    let (socket, _response) =
        tungstenite::client_tls(request, stream).map_err(|error| match error {
            HandshakeError::Failure(error) => format!("Handshake failed: {error}"),
            HandshakeError::Interrupted(_) => "The server did not answer the handshake".into(),
        })?;

    // From here on a read waits `POLL` at most, so the loop can also send.
    tcp(&socket)?
        .set_read_timeout(Some(POLL))
        .map_err(|error| error.to_string())?;
    Ok(socket)
}

/// The TCP stream under the socket, TLS or not.
///
/// (A `try_clone` of the stream taken before the handshake looks simpler, but on Windows
/// a timeout set through the clone does not reach reads made through the original.)
pub fn tcp(socket: &Socket) -> Result<&TcpStream, String> {
    match socket.get_ref() {
        MaybeTlsStream::Plain(tcp) => Ok(tcp),
        MaybeTlsStream::Rustls(tls) => Ok(tls.get_ref()),
        // The enum grows a variant per TLS feature of tungstenite; only these two are on.
        _ => Err("Unexpected TLS stream".into()),
    }
}

/// Says goodbye: sends a close frame and waits briefly for the server's.
pub fn close(socket: &mut Socket) {
    if socket.close(None).is_err() {
        return;
    }
    for _ in 0..CLOSE_POLLS {
        match socket.read() {
            Err(tungstenite::Error::Io(error)) if is_timeout(error.kind()) => {}
            Ok(_) => {}
            Err(_) => return,
        }
    }
}

/// A read that ran out of time: `WouldBlock` on Unix, `TimedOut` on Windows.
pub fn is_timeout(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

pub fn describe_close(frame: CloseFrame) -> String {
    let code = u16::from(frame.code);
    if frame.reason.is_empty() {
        format!("code {code}")
    } else {
        format!("{} (code {code})", frame.reason)
    }
}
