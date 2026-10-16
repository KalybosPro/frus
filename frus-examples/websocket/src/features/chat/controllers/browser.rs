//! The Web's [`Connection`]: the browser's own `WebSocket`, behind the same API as the
//! native one — [`open`](Connection::open), [`send`](Connection::send),
//! [`close`](Connection::close), and events through `on_event`.
//!
//! A browser does the networking itself and calls back on the page's only thread, so
//! there is no socket thread and no pump here: the socket's `onopen`, `onmessage` and
//! `onclose` handlers call `on_event` directly.
//!
//! One thing is different. Those handlers run *between* frames, from the browser's event
//! loop, and the shell only looks for changed state when it draws a frame. Work a
//! component is waiting on keeps frames coming, so the page waits on a [`Watch`] — a
//! future that ends when the socket does (see `ChatState::watch`) — and every event is on
//! screen at the next frame.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{BinaryType, CloseEvent, MessageEvent, WebSocket};

use super::connection::Event;

/// The JavaScript functions the socket calls. They must live as long as the socket can
/// call them, and be let go once it cannot.
struct Handlers {
    _open: Closure<dyn FnMut()>,
    _message: Closure<dyn FnMut(MessageEvent)>,
    _error: Closure<dyn FnMut()>,
    _close: Closure<dyn FnMut(CloseEvent)>,
}

struct Inner {
    /// `None` for an address the browser refused outright.
    socket: Option<WebSocket>,
    handlers: RefCell<Option<Handlers>>,
    /// Set by `onerror`: the `onclose` that always follows then reports a failure.
    errored: Cell<bool>,
    /// The socket is finished with: closed, failed, or let go by the page.
    done: Cell<bool>,
    /// Who to tell when it is.
    waker: RefCell<Option<Waker>>,
}

impl Inner {
    /// Ends the connection's life on our side: no more events, the [`Watch`] completes.
    fn finish(&self) {
        if self.done.replace(true) {
            return;
        }
        if let Some(socket) = &self.socket {
            socket.set_onopen(None);
            socket.set_onmessage(None);
            socket.set_onerror(None);
            socket.set_onclose(None);
        }
        // A handler may be running right now (this is often called from `onclose`), and a
        // closure must not be freed while it runs: let the handlers go once it returns.
        let handlers = self.handlers.borrow_mut().take();
        wasm_bindgen_futures::spawn_local(async move { drop(handlers) });
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }
}

/// A live connection to a server, through the browser.
pub(crate) struct Connection {
    inner: Rc<Inner>,
}

impl Connection {
    /// Starts connecting to `url` and returns at once; the outcome arrives as an [`Event`]
    /// through `on_event`, on the page's thread.
    pub(crate) fn open(url: &str, on_event: impl Fn(Event) + 'static) -> Self {
        let socket = match WebSocket::new(url) {
            Ok(socket) => socket,
            // A malformed address throws at once. Report it like any failure, but not
            // from inside this call: the caller is in the middle of changing the state.
            Err(error) => return Self::failed(describe(&error), Rc::new(on_event)),
        };
        // Binary messages as an `ArrayBuffer` rather than a `Blob`: its length can be
        // read without waiting.
        socket.set_binary_type(BinaryType::Arraybuffer);

        let on_event: Rc<dyn Fn(Event)> = Rc::new(on_event);
        let inner = Rc::new(Inner {
            socket: Some(socket.clone()),
            handlers: RefCell::new(None),
            errored: Cell::new(false),
            done: Cell::new(false),
            waker: RefCell::new(None),
        });

        let open = {
            let on_event = on_event.clone();
            Closure::<dyn FnMut()>::new(move || on_event(Event::Connected))
        };
        let message = {
            let on_event = on_event.clone();
            Closure::<dyn FnMut(MessageEvent)>::new(move |message: MessageEvent| {
                let data = message.data();
                let text = match data.as_string() {
                    Some(text) => text,
                    None => match data.dyn_ref::<js_sys::ArrayBuffer>() {
                        Some(bytes) => format!("[{} bytes of binary]", bytes.byte_length()),
                        None => "[a message that is not text]".to_string(),
                    },
                };
                on_event(Event::Received(text));
            })
        };
        // The browser says nothing about *why* (on purpose: a page must not learn what
        // the network looks like); the `onclose` that follows carries the code.
        let error = {
            let inner = Rc::downgrade(&inner);
            Closure::<dyn FnMut()>::new(move || {
                if let Some(inner) = inner.upgrade() {
                    inner.errored.set(true);
                }
            })
        };
        let close = {
            let inner = Rc::downgrade(&inner);
            Closure::<dyn FnMut(CloseEvent)>::new(move |close: CloseEvent| {
                let Some(inner) = inner.upgrade() else {
                    return;
                };
                let failed = inner.errored.get() || !close.was_clean();
                // Finished first: `on_event` may drop the `Connection`, which is fine
                // once there is nothing left for it to do.
                inner.finish();
                on_event(if failed {
                    Event::Failed(format!("The connection failed (code {})", close.code()))
                } else {
                    let reason = close.reason();
                    Event::Closed(Some(if reason.is_empty() {
                        format!("code {}", close.code())
                    } else {
                        format!("{reason} (code {})", close.code())
                    }))
                });
            })
        };

        socket.set_onopen(Some(open.as_ref().unchecked_ref()));
        socket.set_onmessage(Some(message.as_ref().unchecked_ref()));
        socket.set_onerror(Some(error.as_ref().unchecked_ref()));
        socket.set_onclose(Some(close.as_ref().unchecked_ref()));
        *inner.handlers.borrow_mut() = Some(Handlers {
            _open: open,
            _message: message,
            _error: error,
            _close: close,
        });

        Self { inner }
    }

    /// A connection that never opened: it reports `error` just after this call returns.
    fn failed(error: String, on_event: Rc<dyn Fn(Event)>) -> Self {
        frus::host::after(std::time::Duration::ZERO, move || {
            on_event(Event::Failed(error.clone()))
        });
        let inner = Rc::new(Inner {
            socket: None,
            handlers: RefCell::new(None),
            errored: Cell::new(true),
            done: Cell::new(true),
            waker: RefCell::new(None),
        });
        Self { inner }
    }

    /// Sends `text`. Only called while connected; if the socket has gone meanwhile, its
    /// event is on its way and says why.
    pub(crate) fn send(&self, text: String) {
        if let (false, Some(socket)) = (self.inner.done.get(), &self.inner.socket) {
            let _ = socket.send_with_str(&text);
        }
    }

    /// Closes the connection politely. Dropping the `Connection` does the same.
    pub(crate) fn close(&self) {
        if let (false, Some(socket)) = (self.inner.done.get(), &self.inner.socket) {
            let _ = socket.close();
        }
        self.inner.finish();
    }

    /// A future that ends when this connection does.
    pub(crate) fn watch(&self) -> Watch {
        Watch(Some(self.inner.clone()))
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.close();
    }
}

/// Ends when the connection does; while it waits, the shell keeps drawing frames, which
/// is what shows the events the browser delivers between them. `Watch::none()` is
/// already over: for when there is no connection.
pub(crate) struct Watch(Option<Rc<Inner>>);

impl Watch {
    pub(crate) fn none() -> Self {
        Watch(None)
    }
}

impl Future for Watch {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        match &self.0 {
            Some(inner) if !inner.done.get() => {
                *inner.waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
            _ => Poll::Ready(()),
        }
    }
}

fn describe(error: &wasm_bindgen::JsValue) -> String {
    error
        .dyn_ref::<js_sys::Error>()
        .map(|error| String::from(error.message()))
        .or_else(|| error.as_string())
        .unwrap_or_else(|| "Not a WebSocket address".to_string())
}
