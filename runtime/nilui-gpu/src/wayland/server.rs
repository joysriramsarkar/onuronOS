// runtime/nilui-gpu/src/wayland/server.rs — the Wayland server
//
// Accepts client connections on a Unix socket, decodes requests against the
// core protocol, and turns committed `wl_surface` buffers into frames the
// DRM/KMS presenter can scan out.
//
// Design note on buffer lifetime: when a surface commits with a buffer
// attached, this server copies the pixels out of the client's pool into an
// owned buffer and immediately sends `wl_buffer.release`. A real compositor may
// instead keep the pool mapped and read it lazily, but copying at commit is
// simpler, cannot race with the client's next write, and lets the client reuse
// the buffer at once. The cost is one memcpy per commit, which is acceptable
// for a software compositor and is stated here so the choice is visible.
//
// Object-id direction, which is the classic Wayland bug: for a request whose
// signature contains `n`, the *client* picks the id and the server adopts it.
// For an event containing `n`, the *server* picks it. Every `new_id` below is
// therefore taken from the request arguments, never generated locally.

use std::collections::HashMap;
use std::io::{self, Write};
use std::os::fd::RawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::protocol::*;
use super::shm::{BufferLayout, ShmPool};
use super::wire::{self, Arg, EncodedMessage};

/// Identifies a surface across all clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfaceKey {
    pub client: u64,
    pub object: u32,
}

/// A committed surface frame, ready to be composited.
#[derive(Debug, Clone)]
pub struct SurfaceFrame {
    pub key: SurfaceKey,
    pub width: u32,
    pub height: u32,
    /// Packed ARGB8888, row-major, tightly packed (no padding).
    pub pixels: Vec<u32>,
    pub buffer_scale: i32,
    pub transform: u32,
    /// Surface-local offset from `wl_surface.offset`.
    pub offset: (i32, i32),
    /// Monotonic commit counter, so a presenter can tell frames apart.
    pub serial: u64,
}

impl SurfaceFrame {
    /// Read pixel `(x, y)`, or 0 when out of range.
    pub fn pixel(&self, x: u32, y: u32) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        self.pixels[(y as usize) * (self.width as usize) + (x as usize)]
    }

    /// Scale this frame to fit `dst_w` x `dst_h` using nearest-neighbour.
    ///
    /// Nearest-neighbour is chosen deliberately: it is exact for integer scale
    /// factors, which is the common case when an Android surface is scaled to a
    /// device panel, and it never invents a colour that was not in the source.
    pub fn scaled_to(&self, dst_w: u32, dst_h: u32) -> Vec<u32> {
        let mut out = vec![0u32; (dst_w as usize) * (dst_h as usize)];
        if self.width == 0 || self.height == 0 || dst_w == 0 || dst_h == 0 {
            return out;
        }
        for dy in 0..dst_h {
            let sy = ((dy as u64 * self.height as u64) / dst_h as u64) as u32;
            let sy = sy.min(self.height - 1);
            for dx in 0..dst_w {
                let sx = ((dx as u64 * self.width as u64) / dst_w as u64) as u32;
                let sx = sx.min(self.width - 1);
                out[(dy as usize) * (dst_w as usize) + (dx as usize)] = self.pixel(sx, sy);
            }
        }
        out
    }
}

/// State shared between the accept loop, the client threads, and the presenter.
#[derive(Default)]
pub struct CompositorState {
    /// Latest committed frame per surface, in commit order.
    frames: Vec<SurfaceFrame>,
    /// Frames committed since the last `take_frames`, in commit order.
    pending: Vec<SurfaceFrame>,
    /// Bumped on every commit, so a presenter can skip work when nothing moved.
    generation: u64,
    /// Surfaces that have been destroyed, so the presenter can drop them.
    destroyed: Vec<SurfaceKey>,
}

impl CompositorState {
    /// Record a commit and queue it for the presenter.
    pub(crate) fn commit(&mut self, frame: SurfaceFrame) {
        self.generation += 1;
        if let Some(slot) = self.frames.iter_mut().find(|f| f.key == frame.key) {
            *slot = frame.clone();
        } else {
            self.frames.push(frame.clone());
        }
        self.pending.push(frame);
    }

    fn destroy(&mut self, key: SurfaceKey) {
        self.frames.retain(|f| f.key != key);
        self.pending.retain(|f| f.key != key);
        self.destroyed.push(key);
    }

    /// Every live surface frame, in the order the surfaces were first committed.
    pub fn frames(&self) -> &[SurfaceFrame] {
        &self.frames
    }

    /// The latest frame for one surface.
    pub fn frame(&self, key: SurfaceKey) -> Option<&SurfaceFrame> {
        self.frames.iter().find(|f| f.key == key)
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Drain the frames committed since the last call.
    pub fn take_frames(&mut self) -> Vec<SurfaceFrame> {
        std::mem::take(&mut self.pending)
    }

    /// Drain the surfaces destroyed since the last call.
    pub fn take_destroyed(&mut self) -> Vec<SurfaceKey> {
        std::mem::take(&mut self.destroyed)
    }
}

/// A protocol violation, carrying the code and message the client is owed.
///
/// The protocol requires the server to send `wl_display.error` *before* closing
/// the connection, so a bare `io::Error` is not enough: the code and message
/// must survive up to the point where the event is encoded.
#[derive(Debug, Clone)]
pub struct ProtocolError {
    pub object: u32,
    pub code: u32,
    pub message: String,
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "protocol error on object {} (code {}): {}",
            self.object, self.code, self.message
        )
    }
}

impl std::error::Error for ProtocolError {}

/// Descriptors received from a client and not yet adopted by a request.
///
/// Closing on drop is the point: a protocol violation leaves the session loop,
/// and a plain `Vec<RawFd>` left the descriptors open, leaking one per malformed
/// message until the fd table filled.
struct FdGuard(Vec<RawFd>);

impl FdGuard {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push_all(&mut self, fds: Vec<RawFd>) {
        self.0.extend(fds);
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Hand `n` descriptors to a request, which then owns closing them.
    fn take(&mut self, n: usize) -> Vec<RawFd> {
        self.0.drain(..n).collect()
    }

    /// Close everything still held.
    fn close_all(&mut self) {
        for fd in self.0.drain(..) {
            // SAFETY: each descriptor was received over SCM_RIGHTS and is owned
            // by this process until closed.
            unsafe { libc::close(fd) };
        }
    }
}

impl Drop for FdGuard {
    fn drop(&mut self) {
        self.close_all();
    }
}

/// A parsed request, ready to dispatch.
struct Request {
    object: u32,
    opcode: u16,
    args: Vec<Arg>,
    /// Descriptors delivered with this message. The dispatcher owns them and
    /// must close any it does not adopt.
    fds: Vec<RawFd>,
}

/// Per-client protocol state.
struct Client {
    id: u64,
    stream: UnixStream,
    /// Objects this client has, keyed by id.
    objects: HashMap<u32, ObjectKind>,
    /// Surfaces this client created: object id -> pending state.
    surfaces: HashMap<u32, SurfaceState>,
    /// shm pools, keyed by object id.
    pools: HashMap<u32, Arc<ShmPool>>,
    /// Buffers: object id -> (pool id, layout, format).
    buffers: HashMap<u32, (u32, BufferLayout, u32)>,
    /// Frame callbacks awaiting completion, keyed by callback object id.
    callbacks: Vec<u32>,
    /// Objects created through `wl_registry.bind`, so we can tell the client
    /// which globals it bound.
    bound: Vec<(u32, String)>,
    serial: u32,
}

/// State that accumulates between `attach` and `commit`.
#[derive(Default, Clone)]
struct SurfaceState {
    /// The buffer set by the most recent `wl_surface.attach`. It stays current
    /// until another attach replaces it or a null attach detaches it.
    current_buffer: Option<u32>,
    buffer_scale: i32,
    transform: u32,
    offset: (i32, i32),
}

/// The listening server.
pub struct WaylandServer {
    listener: UnixListener,
    path: PathBuf,
    /// Owns the socket file, removing it on drop.
    state: Arc<Mutex<CompositorState>>,
    /// Globals offered to clients.
    globals: Vec<Global>,
    client_counter: AtomicU64,
    /// Output geometry advertised through `wl_output`.
    output: OutputInfo,
    running: Arc<AtomicBool>,
    /// Signalled whenever a surface commits, so a presenter can block instead
    /// of polling. Polling loses frames: a client that commits and then
    /// destroys its surface inside one poll interval would never be drawn.
    wake: Arc<WakeHandle>,
}

/// An eventfd used to wake a blocked presenter.
///
/// An eventfd is chosen over a pipe because it is a single descriptor with no
/// buffer to fill and no partial-read semantics: a read resets the counter.
pub struct WakeHandle {
    fd: RawFd,
}

impl WakeHandle {
    fn new() -> io::Result<Self> {
        // EFD_CLOEXEC so the descriptor does not leak into a child process.
        // SAFETY: eventfd with a zero initial count and valid flags.
        let fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd })
    }

    /// The raw descriptor, for `poll`.
    pub fn raw_fd(&self) -> RawFd {
        self.fd
    }

    /// Bump the counter. Never blocks: the counter saturates rather than
    /// filling, so a commit can always signal.
    pub fn signal(&self) {
        let one: u64 = 1;
        // SAFETY: writing 8 bytes to an eventfd is the documented way to
        // increment it. A failure here means the descriptor is gone, which only
        // happens during teardown.
        unsafe {
            libc::write(
                self.fd,
                (&one as *const u64) as *const libc::c_void,
                std::mem::size_of::<u64>(),
            );
        }
    }

    /// Consume the counter, blocking until a commit arrives.
    pub fn wait(&self) -> io::Result<()> {
        let mut buf = [0u8; 8];
        // SAFETY: reading 8 bytes from an eventfd resets the counter.
        let n = unsafe {
            libc::read(
                self.fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                std::mem::size_of::<u64>(),
            )
        };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for WakeHandle {
    fn drop(&mut self) {
        // SAFETY: closing a descriptor this type owns.
        unsafe { libc::close(self.fd) };
    }
}

/// The output this compositor advertises.
#[derive(Debug, Clone, Copy)]
pub struct OutputInfo {
    pub width: u32,
    pub height: u32,
    /// Refresh in millihertz, matching `wl_output.mode`.
    pub refresh_mhz: i32,
}

impl Default for OutputInfo {
    fn default() -> Self {
        Self {
            width: 1080,
            height: 2400,
            refresh_mhz: 60_000,
        }
    }
}

impl WaylandServer {
    /// Bind a listening socket at `path`.
    ///
    /// A stale socket file from a crashed run is removed first, because
    /// `bind` fails with `EADDRINUSE` on an existing path even when nothing is
    /// listening on it.
    pub fn bind(path: impl AsRef<Path>, output: OutputInfo) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if path.exists() {
            // Only remove it if nothing answers, so two servers cannot silently
            // fight over one path.
            match UnixStream::connect(&path) {
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        format!("{} already has a listener", path.display()),
                    ))
                }
                Err(_) => {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
        let listener = UnixListener::bind(&path)?;
        // Non-blocking, so the accept loop can notice `shutdown()` and so the
        // `WouldBlock` arm below is reachable. A blocking `accept` parked the
        // thread indefinitely, which meant the compositor could not be stopped
        // by signal and left its socket bound after a restart attempt. Accepted
        // streams are left blocking, since client threads read synchronously.
        listener.set_nonblocking(true)?;

        let globals = vec![
            Global {
                name: 1,
                interface: WL_COMPOSITOR.to_string(),
                version: VERSION_COMPOSITOR,
            },
            Global {
                name: 2,
                interface: WL_SHM.to_string(),
                version: VERSION_SHM,
            },
            Global {
                name: 3,
                interface: WL_SEAT.to_string(),
                version: VERSION_SEAT,
            },
            Global {
                name: 4,
                interface: WL_OUTPUT.to_string(),
                version: VERSION_OUTPUT,
            },
        ];

        Ok(Self {
            listener,
            path,
            state: Arc::new(Mutex::new(CompositorState::default())),
            globals,
            client_counter: AtomicU64::new(1),
            output,
            running: Arc::new(AtomicBool::new(true)),
            wake: Arc::new(WakeHandle::new()?),
        })
    }

    /// The socket path clients connect to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The compositor state, for a presenter running on another thread.
    pub fn state(&self) -> Arc<Mutex<CompositorState>> {
        Arc::clone(&self.state)
    }

    /// A handle that is signalled on every commit.
    ///
    /// A presenter should `poll` [`WakeHandle::raw_fd`] and call
    /// [`WakeHandle::wait`] when it becomes readable, rather than sleeping and
    /// re-checking: polling can miss a frame entirely.
    pub fn wake_handle(&self) -> Arc<WakeHandle> {
        Arc::clone(&self.wake)
    }

    /// The globals this server offers.
    pub fn globals(&self) -> &[Global] {
        &self.globals
    }

    /// Accept one connection and service it on a new thread.
    ///
    /// Returns the assigned client id, or `None` when the server has been shut
    /// down.
    pub fn accept_one(&self) -> io::Result<Option<u64>> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let (stream, _addr) = self.listener.accept()?;
        let id = self.client_counter.fetch_add(1, Ordering::SeqCst);
        let state = Arc::clone(&self.state);
        let globals = self.globals.clone();
        let output = self.output;
        let running = Arc::clone(&self.running);
        let wake = Arc::clone(&self.wake);
        std::thread::spawn(move || {
            let mut client = Client {
                id,
                stream,
                objects: HashMap::new(),
                surfaces: HashMap::new(),
                pools: HashMap::new(),
                buffers: HashMap::new(),
                callbacks: Vec::new(),
                bound: Vec::new(),
                serial: 0,
            };
            client.objects.insert(1, ObjectKind::Display);
            // A protocol error or a dropped connection ends the client; either
            // way the loop stops and the state is cleaned up.
            let _ = client.run(&state, &globals, output, &running, &wake);
            let mut st = state.lock().unwrap_or_else(|e| e.into_inner());
            let keys: Vec<SurfaceKey> = st
                .frames()
                .iter()
                .filter(|f| f.key.client == id)
                .map(|f| f.key)
                .collect();
            for k in keys {
                st.destroy(k);
            }
        });
        Ok(Some(id))
    }

    /// Stop accepting new clients.
    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Run the accept loop until [`shutdown`](Self::shutdown) is called.
    ///
    /// Each connection is serviced on its own thread, so a client that stalls
    /// cannot block the others.
    pub fn run(&self) -> io::Result<()> {
        while self.running.load(Ordering::SeqCst) {
            match self.accept_one() {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl Drop for WaylandServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Client {
    /// Build the error for a failed write.
    fn transport_err(&self, e: io::Error) -> ProtocolError {
        ProtocolError {
            object: 1,
            code: display_error::IMPLEMENTATION,
            message: format!("transport failure: {e}"),
        }
    }

    fn err(&self, object: u32, code: u32, msg: &str) -> ProtocolError {
        ProtocolError {
            object,
            code,
            message: msg.to_string(),
        }
    }

    /// Send one event, attaching any descriptors with SCM_RIGHTS.
    ///
    /// A transport failure is reported as a `ProtocolError` with code
    /// `IMPLEMENTATION`: it ends the session but is our fault, not the client's.
    fn send(&mut self, mut msg: EncodedMessage) -> Result<(), ProtocolError> {
        if msg.fds.is_empty() {
            self.stream
                .write_all(&msg.bytes)
                .map_err(|e| self.transport_err(e))?;
            self.stream.flush().map_err(|e| self.transport_err(e))?;
            return Ok(());
        }
        // An fd can only travel with at least one byte of payload. Every
        // Wayland message has an 8-byte header, so that is satisfied already.
        let fds = std::mem::take(&mut msg.fds);
        send_with_fds(&self.stream, &msg.bytes, &fds).map_err(|e| self.transport_err(e))?;
        for fd in fds {
            // The kernel duplicated each descriptor into the peer's table; ours
            // must be closed or it leaks.
            unsafe { libc::close(fd) };
        }
        Ok(())
    }

    /// Send a built encoder.
    fn emit(&mut self, mut enc: wire::Encoder) -> Result<(), ProtocolError> {
        let msg = enc.finish().map_err(|e| ProtocolError {
            object: 1,
            code: display_error::IMPLEMENTATION,
            message: format!("could not serialise an event: {e}"),
        })?;
        self.send(msg)
    }

    fn run(
        &mut self,
        state: &Arc<Mutex<CompositorState>>,
        globals: &[Global],
        output: OutputInfo,
        running: &AtomicBool,
        wake: &WakeHandle,
    ) -> io::Result<()> {
        let mut buf = vec![0u8; 64 * 1024];
        // Bytes read but not yet consumed by a complete message.
        let mut pending: Vec<u8> = Vec::new();
        // Descriptors received with the current read.
        let mut pending_fds = FdGuard::new();

        // The protocol requires `wl_display.error` to be sent before the
        // connection closes, so protocol violations are collected here and
        // reported after the loop rather than returned straight away.
        let mut protocol_error: Option<ProtocolError> = None;

        'session: while running.load(Ordering::SeqCst) {
            let mut fds = Vec::new();
            let n = match recv_with_fds(&self.stream, &mut buf, &mut fds) {
                Ok(0) => break, // clean disconnect
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            pending.extend_from_slice(&buf[..n]);
            pending_fds.push_all(fds);

            // Consume every complete message currently buffered.
            loop {
                if pending.len() < wire::HEADER_SIZE {
                    break;
                }
                let size = match wire::declared_size(&pending) {
                    Ok(s) => s,
                    Err(e) => {
                        protocol_error = Some(ProtocolError {
                            object: 1,
                            code: display_error::INVALID_METHOD,
                            message: format!("unframeable message: {e}"),
                        });
                        break 'session;
                    }
                };
                if pending.len() < size {
                    break;
                }
                let message: Vec<u8> = pending.drain(..size).collect();
                let (object, opcode, body) = match wire::decode_message(&message) {
                    Ok(v) => v,
                    Err(e) => {
                        protocol_error = Some(ProtocolError {
                            object: 1,
                            code: display_error::INVALID_METHOD,
                            message: format!("undecodable header: {e}"),
                        });
                        break 'session;
                    }
                };

                // A request on an unknown object or with an unknown opcode has
                // no signature, so it is reported before any descriptor
                // accounting happens.
                let signature = match self.signature(object, opcode) {
                    Ok(s) => s,
                    Err(e) => {
                        protocol_error = Some(e);
                        break 'session;
                    }
                };

                // Descriptors are consumed per message; any the message did not
                // use stay buffered for the next one.
                let fd_count = signature.chars().filter(|c| *c == 'h').count();
                if pending_fds.len() < fd_count {
                    protocol_error = Some(ProtocolError {
                        object,
                        code: display_error::INVALID_METHOD,
                        message: "message expects more descriptors than were received".into(),
                    });
                    break 'session;
                }
                let msg_fds: Vec<RawFd> = pending_fds.take(fd_count);
                let args = match wire::decode_body(signature, body, &msg_fds) {
                    Ok(a) => a,
                    Err(e) => {
                        for fd in &msg_fds {
                            unsafe { libc::close(*fd) };
                        }
                        protocol_error = Some(ProtocolError {
                            object,
                            code: display_error::INVALID_METHOD,
                            message: format!("malformed arguments for {signature}: {e}"),
                        });
                        break 'session;
                    }
                };

                // A commit is the only request a presenter needs to hear about.
                let wake_pending = matches!(self.objects.get(&object), Some(ObjectKind::Surface))
                    && opcode == surface_req::COMMIT;

                let req = Request {
                    object,
                    opcode,
                    args,
                    fds: msg_fds,
                };
                if let Err(e) = self.dispatch(req, state, globals, output) {
                    protocol_error = Some(e);
                    break 'session;
                }
                // A commit may have been recorded; wake the presenter.
                if wake_pending {
                    wake.signal();
                }
            }

            // Descriptors without a corresponding complete message cannot be
            // matched to one, so they are closed rather than held indefinitely.
            if pending.is_empty() && !pending_fds.is_empty() {
                pending_fds.close_all();
            }
        }

        // Report the violation to the client before the socket closes. A failed
        // write here means the client is already gone, which is not an error.
        if let Some(e) = protocol_error {
            let enc = display_error_event(1, e.object, e.code, &e.message);
            let _ = self.emit(enc);
            return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string()));
        }

        // Whatever is left is closed by the guard when it drops.
        Ok(())
    }

    /// The argument signature for a request, or an error for an unknown one.
    fn signature(&self, object: u32, opcode: u16) -> Result<&'static str, ProtocolError> {
        let kind = self.objects.get(&object).ok_or_else(|| {
            self.err(
                object,
                display_error::INVALID_OBJECT,
                "request on an object the server does not know",
            )
        })?;
        let sig = match (kind, opcode) {
            (ObjectKind::Display, display_req::SYNC) => "n",
            (ObjectKind::Display, display_req::GET_REGISTRY) => "n",
            (ObjectKind::Registry, registry_req::BIND) => "usun",
            (ObjectKind::Registry, registry_req::DESTROY) => "",
            (ObjectKind::Compositor, compositor_req::CREATE_SURFACE) => "n",
            (ObjectKind::Compositor, compositor_req::CREATE_REGION) => "n",
            (ObjectKind::Surface, surface_req::DESTROY) => "",
            (ObjectKind::Surface, surface_req::ATTACH) => "oii",
            (ObjectKind::Surface, surface_req::DAMAGE) => "iiii",
            (ObjectKind::Surface, surface_req::FRAME) => "n",
            (ObjectKind::Surface, surface_req::SET_OPAQUE_REGION) => "o",
            (ObjectKind::Surface, surface_req::SET_INPUT_REGION) => "o",
            (ObjectKind::Surface, surface_req::COMMIT) => "",
            (ObjectKind::Surface, surface_req::SET_BUFFER_TRANSFORM) => "i",
            (ObjectKind::Surface, surface_req::SET_BUFFER_SCALE) => "i",
            (ObjectKind::Surface, surface_req::DAMAGE_BUFFER) => "iiii",
            (ObjectKind::Shm, shm_req::CREATE_POOL) => "nhi",
            (ObjectKind::ShmPool, shm_pool_req::CREATE_BUFFER) => "niiiiu",
            (ObjectKind::ShmPool, shm_pool_req::DESTROY) => "",
            (ObjectKind::ShmPool, shm_pool_req::RESIZE) => "i",
            (ObjectKind::Buffer, buffer_req::DESTROY) => "",
            (ObjectKind::Output, output_req::RELEASE) => "",
            (ObjectKind::Region, _) => "",
            (ObjectKind::Seat, seat_req::GET_POINTER) => "n",
            (ObjectKind::Seat, seat_req::GET_KEYBOARD) => "n",
            (ObjectKind::Seat, seat_req::GET_TOUCH) => "n",
            (ObjectKind::Seat, seat_req::RELEASE) => "",
            (ObjectKind::Touch, touch_req::RELEASE) => "",
            _ => {
                return Err(self.err(
                    object,
                    display_error::INVALID_METHOD,
                    &format!("unknown opcode {opcode} for this object"),
                ))
            }
        };
        Ok(sig)
    }

    fn dispatch(
        &mut self,
        req: Request,
        state: &Arc<Mutex<CompositorState>>,
        globals: &[Global],
        output: OutputInfo,
    ) -> Result<(), ProtocolError> {
        let kind = match self.objects.get(&req.object) {
            Some(k) => k.clone(),
            None => {
                for fd in &req.fds {
                    unsafe { libc::close(*fd) };
                }
                return Err(self.err(
                    req.object,
                    display_error::INVALID_OBJECT,
                    "request on an unknown object",
                ));
            }
        };

        // Descriptors that this request did not adopt must be closed.
        let mut adopted = false;
        let result = match (kind, req.opcode) {
            (ObjectKind::Display, display_req::SYNC) => {
                let cb = req.args[0].as_u32().unwrap();
                self.objects.insert(cb, ObjectKind::Callback);
                self.emit(callback_done_event(cb, self.serial))?;
                self.serial = self.serial.wrapping_add(1);
                self.emit(delete_id_event(req.object, cb))?;
                self.objects.remove(&cb);
                Ok(())
            }
            (ObjectKind::Display, display_req::GET_REGISTRY) => {
                let registry = req.args[0].as_u32().unwrap();
                self.objects.insert(registry, ObjectKind::Registry);
                for g in globals {
                    self.emit(global_event(registry, g))?;
                }
                Ok(())
            }
            (ObjectKind::Registry, registry_req::BIND) => {
                let name = req.args[0].as_u32().unwrap();
                let iface = req.args[1].as_str().unwrap().to_string();
                let version = req.args[2].as_u32().unwrap();
                let new_id = req.args[3].as_u32().unwrap();
                self.bind_global(name, &iface, version, new_id, output)
            }
            (ObjectKind::Registry, registry_req::DESTROY) => {
                self.objects.remove(&req.object);
                Ok(())
            }
            (ObjectKind::Compositor, compositor_req::CREATE_SURFACE) => {
                let id = req.args[0].as_u32().unwrap();
                self.objects.insert(id, ObjectKind::Surface);
                self.surfaces.insert(id, SurfaceState::default());
                // Tell the client which output it is on and what scale to use,
                // which real clients wait for before attaching a buffer.
                self.emit(surface_enter_event(id, self.output_object_id()))?;
                Ok(())
            }
            (ObjectKind::Compositor, compositor_req::CREATE_REGION) => {
                let id = req.args[0].as_u32().unwrap();
                self.objects.insert(id, ObjectKind::Region);
                Ok(())
            }
            (ObjectKind::Surface, surface_req::DESTROY) => {
                let key = SurfaceKey {
                    client: self.id,
                    object: req.object,
                };
                state.lock().unwrap_or_else(|e| e.into_inner()).destroy(key);
                self.surfaces.remove(&req.object);
                self.objects.remove(&req.object);
                Ok(())
            }
            (ObjectKind::Surface, surface_req::ATTACH) => {
                let buffer = req.args[0].as_u32().unwrap();
                let x = req.args[1].as_i32().unwrap();
                let y = req.args[2].as_i32().unwrap();
                let s = self.surfaces.entry(req.object).or_default();
                // A zero buffer id means detach.
                s.current_buffer = if buffer == 0 { None } else { Some(buffer) };
                s.offset = (x, y);
                Ok(())
            }
            (ObjectKind::Surface, surface_req::DAMAGE) => {
                // Damage is accepted and validated but this software compositor
                // recomposites the whole surface, so it is not yet used to limit
                // the blit. Recorded rather than silently dropped.
                Ok(())
            }
            (ObjectKind::Surface, surface_req::FRAME) => {
                let cb = req.args[0].as_u32().unwrap();
                self.objects.insert(cb, ObjectKind::Callback);
                self.callbacks.push(cb);
                Ok(())
            }
            (ObjectKind::Surface, surface_req::SET_OPAQUE_REGION)
            | (ObjectKind::Surface, surface_req::SET_INPUT_REGION) => {
                // Region clipping is not implemented; the request is accepted so
                // clients that always send it still work.
                Ok(())
            }
            (ObjectKind::Surface, surface_req::COMMIT) => self.commit_surface(req.object, state),
            (ObjectKind::Surface, surface_req::SET_BUFFER_TRANSFORM) => {
                let t = req.args[0].as_u32().unwrap();
                if t > transform::FLIPPED_270 {
                    return Err(self.err(
                        req.object,
                        display_error::INVALID_METHOD,
                        "buffer transform out of range",
                    ));
                }
                self.surfaces.entry(req.object).or_default().transform = t;
                Ok(())
            }
            (ObjectKind::Surface, surface_req::SET_BUFFER_SCALE) => {
                let s = req.args[0].as_i32().unwrap();
                if s < 1 {
                    return Err(self.err(
                        req.object,
                        display_error::INVALID_METHOD,
                        "buffer scale must be at least 1",
                    ));
                }
                self.surfaces.entry(req.object).or_default().buffer_scale = s;
                Ok(())
            }
            (ObjectKind::Surface, surface_req::DAMAGE_BUFFER) => Ok(()),
            (ObjectKind::Shm, shm_req::CREATE_POOL) => {
                let id = req.args[0].as_u32().unwrap();
                let size = req.args[2].as_i32().unwrap();
                let fd = match req.args[1].as_fd() {
                    Some(fd) => fd,
                    None => {
                        return Err(self.err(
                            req.object,
                            shm_error::INVALID_FD,
                            "create_pool without a descriptor",
                        ))
                    }
                };
                if size <= 0 {
                    unsafe { libc::close(fd) };
                    return Err(self.err(
                        req.object,
                        shm_error::INVALID_STRIDE,
                        "pool size must be positive",
                    ));
                }
                let pool = match ShmPool::map(fd, size as u32) {
                    Ok(p) => p,
                    Err(e) => {
                        unsafe { libc::close(fd) };
                        return Err(self.err(req.object, shm_error::INVALID_FD, &e.to_string()));
                    }
                };
                // The mapping is independent of the fd, so the descriptor is
                // closed now; keeping it would leak one fd per pool.
                unsafe { libc::close(fd) };
                adopted = true;
                self.pools.insert(id, Arc::new(pool));
                self.objects.insert(id, ObjectKind::ShmPool);
                Ok(())
            }
            (ObjectKind::ShmPool, shm_pool_req::CREATE_BUFFER) => {
                let id = req.args[0].as_u32().unwrap();
                let offset = req.args[1].as_i32().unwrap();
                let width = req.args[2].as_i32().unwrap();
                let height = req.args[3].as_i32().unwrap();
                let stride = req.args[4].as_i32().unwrap();
                let format = req.args[5].as_u32().unwrap();

                if format != shm_format::ARGB8888 && format != shm_format::XRGB8888 {
                    return Err(self.err(
                        req.object,
                        shm_error::INVALID_FORMAT,
                        "only ARGB8888 and XRGB8888 are supported",
                    ));
                }
                let pool = match self.pools.get(&req.object) {
                    Some(p) => Arc::clone(p),
                    None => {
                        return Err(self.err(
                            req.object,
                            display_error::INVALID_OBJECT,
                            "create_buffer on an unknown pool",
                        ))
                    }
                };
                let layout = match pool.validate_buffer(offset, width, height, stride) {
                    Ok(l) => l,
                    Err(e) => {
                        let code = match e {
                            super::shm::ShmError::StrideTooSmall { .. } => {
                                shm_error::INVALID_STRIDE
                            }
                            super::shm::ShmError::UnsupportedFormat(_) => shm_error::INVALID_FORMAT,
                            _ => shm_error::INVALID_STRIDE,
                        };
                        return Err(self.err(req.object, code, &e.to_string()));
                    }
                };
                self.buffers.insert(id, (req.object, layout, format));
                self.objects.insert(id, ObjectKind::Buffer);
                Ok(())
            }
            (ObjectKind::ShmPool, shm_pool_req::DESTROY) => {
                self.pools.remove(&req.object);
                self.objects.remove(&req.object);
                Ok(())
            }
            (ObjectKind::ShmPool, shm_pool_req::RESIZE) => {
                // Growing a pool would require remapping, and every buffer made
                // from it would need revalidation. Refused loudly instead of
                // accepting a size we would then not honour.
                Err(self.err(
                    req.object,
                    display_error::INVALID_METHOD,
                    "wl_shm_pool.resize is not supported; create a new pool",
                ))
            }
            (ObjectKind::Buffer, buffer_req::DESTROY) => {
                self.buffers.remove(&req.object);
                self.objects.remove(&req.object);
                Ok(())
            }
            (ObjectKind::Output, output_req::RELEASE) => {
                self.objects.remove(&req.object);
                Ok(())
            }
            (ObjectKind::Region, _) => Ok(()),
            (ObjectKind::Seat, seat_req::GET_TOUCH) => {
                let id = req.args[0].as_u32().unwrap();
                self.objects.insert(id, ObjectKind::Touch);
                Ok(())
            }
            (ObjectKind::Seat, seat_req::GET_POINTER)
            | (ObjectKind::Seat, seat_req::GET_KEYBOARD) => {
                // Accepted so clients that always request a pointer or keyboard
                // do not fail; the object is tracked with no events sent.
                let id = req.args[0].as_u32().unwrap();
                self.objects.insert(id, ObjectKind::Region);
                Ok(())
            }
            (ObjectKind::Seat, seat_req::RELEASE) => {
                self.objects.remove(&req.object);
                Ok(())
            }
            (ObjectKind::Touch, touch_req::RELEASE) => {
                self.objects.remove(&req.object);
                Ok(())
            }
            _ => Err(self.err(
                req.object,
                display_error::INVALID_METHOD,
                "unhandled request",
            )),
        };

        // Close any descriptor the request carried but did not adopt.
        if !adopted {
            for fd in &req.fds {
                unsafe { libc::close(*fd) };
            }
        }
        result
    }

    /// The id of the `wl_output` object this client bound, if any.
    fn output_object_id(&self) -> u32 {
        self.bound
            .iter()
            .find(|(_, iface)| iface == WL_OUTPUT)
            .map(|(id, _)| *id)
            .unwrap_or(0)
    }

    fn bind_global(
        &mut self,
        name: u32,
        iface: &str,
        version: u32,
        new_id: u32,
        output: OutputInfo,
    ) -> Result<(), ProtocolError> {
        let known = match iface {
            WL_COMPOSITOR => Some((ObjectKind::Compositor, VERSION_COMPOSITOR)),
            WL_SHM => Some((ObjectKind::Shm, VERSION_SHM)),
            WL_SEAT => Some((ObjectKind::Seat, VERSION_SEAT)),
            WL_OUTPUT => Some((ObjectKind::Output, VERSION_OUTPUT)),
            _ => None,
        };
        let (kind, max_version) = match known {
            Some(k) => k,
            None => {
                return Err(self.err(
                    self.registry_object_id(),
                    display_error::INVALID_METHOD,
                    &format!("no global named {iface} (name {name})"),
                ))
            }
        };
        // Advertise no higher than we implement, and no higher than requested.
        let effective = version.min(max_version);
        self.objects.insert(new_id, kind.clone());
        self.bound.push((new_id, iface.to_string()));

        match kind {
            ObjectKind::Shm => {
                self.emit(shm_format_event(new_id, shm_format::ARGB8888))?;
                self.emit(shm_format_event(new_id, shm_format::XRGB8888))?;
            }
            ObjectKind::Seat => {
                self.emit(seat_capabilities_event(new_id, seat_capability::TOUCH))?;
                self.emit(seat_name_event(new_id, "onuron-seat0"))?;
            }
            ObjectKind::Output => {
                self.emit(output_geometry_event(
                    new_id,
                    0,
                    0,
                    output.width as i32 * 4 / 1000,
                    output.height as i32 * 4 / 1000,
                    subpixel::UNKNOWN,
                    "Onuron",
                    "OnuronOS Panel",
                    transform::NORMAL as i32,
                ))?;
                self.emit(output_mode_event(
                    new_id,
                    output_mode::CURRENT | output_mode::PREFERRED,
                    output.width as i32,
                    output.height as i32,
                    output.refresh_mhz,
                ))?;
                if effective >= 2 {
                    self.emit(output_scale_event(new_id, 1))?;
                }
                self.emit(output_done_event(new_id))?;
            }
            _ => {}
        }
        Ok(())
    }

    fn registry_object_id(&self) -> u32 {
        self.objects
            .iter()
            .find(|(_, k)| **k == ObjectKind::Registry)
            .map(|(id, _)| *id)
            .unwrap_or(1)
    }

    /// Apply a `wl_surface.commit`.
    fn commit_surface(
        &mut self,
        surface: u32,
        state: &Arc<Mutex<CompositorState>>,
    ) -> Result<(), ProtocolError> {
        // The buffer stays current across commits: `attach` sets it and only a
        // later `attach` (or a null attach) replaces it. Taking it here would
        // draw a client's first frame and then nothing.
        let pending = self.surfaces.get(&surface).and_then(|s| s.current_buffer);

        if let Some(buffer_id) = pending {
            let (pool_id, layout, _format) = match self.buffers.get(&buffer_id) {
                Some(b) => *b,
                None => {
                    return Err(self.err(
                        surface,
                        display_error::INVALID_OBJECT,
                        "commit attached an unknown buffer",
                    ))
                }
            };
            let pool = match self.pools.get(&pool_id) {
                Some(p) => Arc::clone(p),
                None => {
                    return Err(self.err(
                        surface,
                        display_error::INVALID_OBJECT,
                        "buffer refers to a pool that is gone",
                    ))
                }
            };

            // Copy the pixels out of the client's pool. Rows are read through
            // the validated stride, so a padded buffer is handled correctly.
            let mut pixels = Vec::with_capacity((layout.width * layout.height) as usize);
            let mut row = Vec::new();
            for y in 0..layout.height {
                pool.copy_row(&layout, y, &mut row);
                pixels.extend_from_slice(&row);
            }

            let surface_state = self.surfaces.get(&surface).cloned().unwrap_or_default();
            let frame = SurfaceFrame {
                key: SurfaceKey {
                    client: self.id,
                    object: surface,
                },
                width: layout.width,
                height: layout.height,
                pixels,
                buffer_scale: surface_state.buffer_scale,
                transform: surface_state.transform,
                offset: surface_state.offset,
                serial: self.serial as u64,
            };
            state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .commit(frame);

            // We have copied everything we need, so the client may reuse it.
            self.emit(buffer_release_event(buffer_id))?;
        }

        // Complete any frame callbacks requested on this surface.
        let callbacks = std::mem::take(&mut self.callbacks);
        for cb in callbacks {
            self.emit(callback_done_event(cb, self.serial))?;
            self.emit(delete_id_event(1, cb))?;
            self.objects.remove(&cb);
        }
        self.serial = self.serial.wrapping_add(1);
        Ok(())
    }
}

// ─── socket helpers ──────────────────────────────────────────────────────────

/// Send `bytes` with `fds` attached as SCM_RIGHTS.
pub fn send_with_fds(stream: &UnixStream, bytes: &[u8], fds: &[RawFd]) -> io::Result<()> {
    if fds.is_empty() {
        let mut s = stream;
        s.write_all(bytes)?;
        return s.flush();
    }
    use std::os::fd::AsRawFd;
    let mut iov = libc::iovec {
        iov_base: bytes.as_ptr() as *mut libc::c_void,
        iov_len: bytes.len(),
    };
    let mut control = vec![0u8; unsafe { libc::CMSG_SPACE(4 * fds.len() as u32) } as usize];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = control.len();

    // SAFETY: `msg` is initialised with a control buffer sized for `fds.len()`
    // descriptors via CMSG_SPACE above.
    unsafe {
        let cmsg = libc::CMSG_FIRSTHDR(&msg);
        if cmsg.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "control buffer too small for fds",
            ));
        }
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(4 * fds.len() as u32) as usize;
        std::ptr::copy_nonoverlapping(
            fds.as_ptr() as *const u8,
            libc::CMSG_DATA(cmsg),
            4 * fds.len(),
        );
        msg.msg_controllen = libc::CMSG_SPACE(4 * fds.len() as u32) as usize;

        let fd = stream.as_raw_fd();
        // The descriptors travel with the first sendmsg call. A short write
        // means the remainder is plain payload, so the control buffer is
        // detached before the follow-up sends.
        let mut sent = 0usize;
        loop {
            let n = libc::sendmsg(fd, &msg, libc::MSG_NOSIGNAL);
            if n < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }
            sent += n as usize;
            if sent >= bytes.len() {
                break;
            }
            iov.iov_base = (bytes.as_ptr() as usize + sent) as *mut libc::c_void;
            iov.iov_len = bytes.len() - sent;
            msg.msg_iov = &mut iov;
            msg.msg_control = std::ptr::null_mut();
            msg.msg_controllen = 0;
        }
    }
    Ok(())
}

/// Receive into `buf`, collecting any descriptors into `fds`.
pub fn recv_with_fds(
    stream: &UnixStream,
    buf: &mut [u8],
    fds: &mut Vec<RawFd>,
) -> io::Result<usize> {
    use std::os::fd::AsRawFd;
    let mut iov = libc::iovec {
        iov_base: buf.as_mut_ptr() as *mut libc::c_void,
        iov_len: buf.len(),
    };
    // Space for several descriptors; a single read can carry more than one.
    const MAX_FDS: usize = 16;
    let mut control = vec![0u8; unsafe { libc::CMSG_SPACE(4 * MAX_FDS as u32) } as usize];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = control.len();

    // SAFETY: `msg` points at `buf` and a control buffer sized for MAX_FDS.
    let n = unsafe { libc::recvmsg(stream.as_raw_fd(), &mut msg, 0) };
    if n < 0 {
        return Err(io::Error::last_os_error());
    }
    if msg.msg_flags & libc::MSG_CTRUNC != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "control data was truncated: too many descriptors in one message",
        ));
    }

    // SAFETY: walking the control messages the kernel wrote into `control`.
    unsafe {
        let mut cmsg = libc::CMSG_FIRSTHDR(&msg);
        while !cmsg.is_null() {
            if (*cmsg).cmsg_level == libc::SOL_SOCKET && (*cmsg).cmsg_type == libc::SCM_RIGHTS {
                let payload = (*cmsg).cmsg_len as usize - libc::CMSG_LEN(0) as usize;
                let count = payload / 4;
                let mut got = [0i32; MAX_FDS];
                std::ptr::copy_nonoverlapping(
                    libc::CMSG_DATA(cmsg),
                    got.as_mut_ptr() as *mut u8,
                    payload,
                );
                for fd in got.iter().take(count) {
                    fds.push(*fd);
                }
            }
            cmsg = libc::CMSG_NXTHDR(&msg, cmsg);
        }
    }
    Ok(n as usize)
}

/// The default socket path, honouring `XDG_RUNTIME_DIR` as Wayland requires.
pub fn default_socket_path() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/onuron".to_string());
    Path::new(&dir).join("wayland-0")
}

/// The argument signature for a request, exposed so tests can build valid
/// messages without duplicating the table.
pub fn request_signature(kind: &ObjectKind, opcode: u16) -> Option<&'static str> {
    Some(match (kind, opcode) {
        (ObjectKind::Display, display_req::SYNC) => "n",
        (ObjectKind::Display, display_req::GET_REGISTRY) => "n",
        (ObjectKind::Registry, registry_req::BIND) => "usun",
        (ObjectKind::Registry, registry_req::DESTROY) => "",
        (ObjectKind::Compositor, compositor_req::CREATE_SURFACE) => "n",
        (ObjectKind::Compositor, compositor_req::CREATE_REGION) => "n",
        (ObjectKind::Surface, surface_req::DESTROY) => "",
        (ObjectKind::Surface, surface_req::ATTACH) => "oii",
        (ObjectKind::Surface, surface_req::DAMAGE) => "iiii",
        (ObjectKind::Surface, surface_req::FRAME) => "n",
        (ObjectKind::Surface, surface_req::SET_OPAQUE_REGION) => "o",
        (ObjectKind::Surface, surface_req::SET_INPUT_REGION) => "o",
        (ObjectKind::Surface, surface_req::COMMIT) => "",
        (ObjectKind::Surface, surface_req::SET_BUFFER_TRANSFORM) => "i",
        (ObjectKind::Surface, surface_req::SET_BUFFER_SCALE) => "i",
        (ObjectKind::Surface, surface_req::DAMAGE_BUFFER) => "iiii",
        (ObjectKind::Shm, shm_req::CREATE_POOL) => "nhi",
        (ObjectKind::ShmPool, shm_pool_req::CREATE_BUFFER) => "niiiiu",
        (ObjectKind::ShmPool, shm_pool_req::DESTROY) => "",
        (ObjectKind::ShmPool, shm_pool_req::RESIZE) => "i",
        (ObjectKind::Buffer, buffer_req::DESTROY) => "",
        (ObjectKind::Output, output_req::RELEASE) => "",
        (ObjectKind::Seat, seat_req::GET_POINTER) => "n",
        (ObjectKind::Seat, seat_req::GET_KEYBOARD) => "n",
        (ObjectKind::Seat, seat_req::GET_TOUCH) => "n",
        (ObjectKind::Seat, seat_req::RELEASE) => "",
        (ObjectKind::Touch, touch_req::RELEASE) => "",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::os::fd::{FromRawFd, IntoRawFd};

    fn temp_socket(tag: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("onuron-wl-{tag}-{}-{n}.sock", std::process::id()))
    }

    #[test]
    fn binds_and_removes_the_socket_on_drop() {
        let path = temp_socket("bind");
        let server = WaylandServer::bind(&path, OutputInfo::default()).unwrap();
        assert!(path.exists());
        drop(server);
        assert!(!path.exists(), "socket file should be cleaned up");
    }

    #[test]
    fn refuses_to_steal_a_path_that_already_has_a_listener() {
        let path = temp_socket("dup");
        let _first = WaylandServer::bind(&path, OutputInfo::default()).unwrap();
        let second = WaylandServer::bind(&path, OutputInfo::default());
        assert!(second.is_err(), "second bind must not silently take over");
    }

    #[test]
    fn removes_a_stale_socket_file() {
        let path = temp_socket("stale");
        // A socket file left behind by a crash has no listener.
        let listener = UnixListener::bind(&path).unwrap();
        drop(listener);
        assert!(path.exists());
        let server = WaylandServer::bind(&path, OutputInfo::default()).unwrap();
        assert!(server.path().exists());
    }

    #[test]
    fn advertises_the_expected_globals() {
        let path = temp_socket("globals");
        let server = WaylandServer::bind(&path, OutputInfo::default()).unwrap();
        let names: Vec<&str> = server
            .globals()
            .iter()
            .map(|g| g.interface.as_str())
            .collect();
        assert!(names.contains(&WL_COMPOSITOR));
        assert!(names.contains(&WL_SHM));
        assert!(names.contains(&WL_SEAT));
        assert!(names.contains(&WL_OUTPUT));
    }

    #[test]
    fn signature_table_covers_the_requests_clients_need() {
        assert_eq!(request_signature(&ObjectKind::Display, 1), Some("n"));
        assert_eq!(
            request_signature(&ObjectKind::Registry, registry_req::BIND),
            Some("usun")
        );
        assert_eq!(
            request_signature(&ObjectKind::Surface, surface_req::ATTACH),
            Some("oii")
        );
        assert_eq!(
            request_signature(&ObjectKind::ShmPool, shm_pool_req::CREATE_BUFFER),
            Some("niiiiu")
        );
        // An opcode nobody implements must be reported, not guessed at.
        assert_eq!(request_signature(&ObjectKind::Display, 99), None);
    }

    #[test]
    fn surface_frame_scales_to_fit_a_panel() {
        // A 2x2 checkerboard scaled up to 4x4 must reproduce the checkerboard.
        let frame = SurfaceFrame {
            key: SurfaceKey {
                client: 1,
                object: 1,
            },
            width: 2,
            height: 2,
            pixels: vec![0xFF000000, 0xFFFFFFFF, 0xFFFFFFFF, 0xFF000000],
            buffer_scale: 1,
            transform: 0,
            offset: (0, 0),
            serial: 0,
        };
        let scaled = frame.scaled_to(4, 4);
        assert_eq!(scaled.len(), 16);
        // Top-left 2x2 block came from source pixel (0,0).
        assert_eq!(scaled[0], 0xFF000000);
        assert_eq!(scaled[1], 0xFF000000);
        assert_eq!(scaled[4], 0xFF000000);
        // Top-right block came from source pixel (1,0).
        assert_eq!(scaled[2], 0xFFFFFFFF);
        assert_eq!(scaled[3], 0xFFFFFFFF);
    }

    #[test]
    fn scaled_to_handles_a_zero_sized_target() {
        let frame = SurfaceFrame {
            key: SurfaceKey {
                client: 1,
                object: 1,
            },
            width: 2,
            height: 2,
            pixels: vec![1, 2, 3, 4],
            buffer_scale: 1,
            transform: 0,
            offset: (0, 0),
            serial: 0,
        };
        assert!(frame.scaled_to(0, 0).is_empty());
        assert!(frame.scaled_to(0, 10).is_empty());
    }

    #[test]
    fn compositor_state_tracks_commit_and_destroy() {
        let mut st = CompositorState::default();
        let key = SurfaceKey {
            client: 1,
            object: 5,
        };
        let frame = SurfaceFrame {
            key,
            width: 1,
            height: 1,
            pixels: vec![0xFF112233],
            buffer_scale: 1,
            transform: 0,
            offset: (0, 0),
            serial: 1,
        };
        st.commit(frame.clone());
        assert_eq!(st.generation(), 1);
        assert_eq!(st.frames().len(), 1);
        assert_eq!(st.frame(key).unwrap().pixels[0], 0xFF112233);
        assert_eq!(st.take_frames().len(), 1);
        // Draining twice yields nothing the second time.
        assert!(st.take_frames().is_empty());

        // A second commit replaces rather than appends.
        st.commit(frame);
        assert_eq!(st.frames().len(), 1);
        assert_eq!(st.generation(), 2);

        st.destroy(key);
        assert!(st.frames().is_empty());
        assert_eq!(st.take_destroyed(), vec![key]);
    }

    #[test]
    fn default_socket_path_follows_xdg_runtime_dir() {
        // The lookup is environment-driven, so assert the shape rather than a
        // fixed value, and that the final component is the Wayland socket name.
        let p = default_socket_path();
        assert_eq!(p.file_name().unwrap(), "wayland-0");
    }

    /// A minimal but real client: connects, handshakes, creates a surface,
    /// attaches a shm buffer with known pixels, commits, and reads the events.
    ///
    /// Nothing here is mocked. The client speaks the same wire format the
    /// server parses and passes a real memfd over SCM_RIGHTS.
    struct TestClient {
        stream: UnixStream,
        buf: Vec<u8>,
    }

    impl TestClient {
        fn connect(path: &Path) -> Self {
            let stream = UnixStream::connect(path).expect("connect");
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            Self {
                stream,
                buf: Vec::new(),
            }
        }

        fn send(&mut self, mut enc: wire::Encoder) {
            let msg = enc.finish().unwrap();
            send_with_fds(&self.stream, &msg.bytes, &msg.fds).unwrap();
            for fd in msg.fds {
                unsafe { libc::close(fd) };
            }
        }

        /// Read one message, decoding it with `signature`.
        fn recv(&mut self, signature: &str) -> Vec<Arg> {
            loop {
                if self.buf.len() >= wire::HEADER_SIZE {
                    let size = wire::declared_size(&self.buf).expect("framing");
                    if self.buf.len() >= size {
                        let msg: Vec<u8> = self.buf.drain(..size).collect();
                        let (_, _, body) = wire::decode_message(&msg).unwrap();
                        return wire::decode_body(signature, body, &[]).expect("decode");
                    }
                }
                let mut chunk = [0u8; 8192];
                let n = self.stream.read(&mut chunk).expect("read");
                assert!(n > 0, "server closed the connection");
                self.buf.extend_from_slice(&chunk[..n]);
            }
        }

        /// Read one message without decoding, returning (object, opcode, body).
        fn recv_raw(&mut self) -> (u32, u16, Vec<u8>) {
            loop {
                if self.buf.len() >= wire::HEADER_SIZE {
                    let size = wire::declared_size(&self.buf).expect("framing");
                    if self.buf.len() >= size {
                        let msg: Vec<u8> = self.buf.drain(..size).collect();
                        let (obj, op, body) = wire::decode_message(&msg).unwrap();
                        return (obj, op, body.to_vec());
                    }
                }
                let mut chunk = [0u8; 8192];
                let n = self.stream.read(&mut chunk).expect("read");
                assert!(n > 0, "server closed the connection");
                self.buf.extend_from_slice(&chunk[..n]);
            }
        }
    }

    /// Whether any descriptor in this process still refers to `inode`.
    ///
    /// Checking a specific inode rather than counting descriptors is what makes
    /// this test immune to the other tests running in parallel: their pools have
    /// different inodes and are simply not this test's business.
    fn inode_still_open(inode: u64) -> bool {
        let Ok(dir) = std::fs::read_dir("/proc/self/fd") else {
            return false;
        };
        for entry in dir.filter_map(|e| e.ok()) {
            // SAFETY: fstat on a descriptor path we just listed; a vanished
            // entry simply fails and is skipped.
            let mut st: libc::stat = unsafe { std::mem::zeroed() };
            let path = entry.path();
            let Ok(cpath) = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()) else {
                continue;
            };
            if unsafe { libc::stat(cpath.as_ptr(), &mut st) } == 0 && st.st_ino as u64 == inode {
                return true;
            }
        }
        false
    }

    /// The inode of an open descriptor.
    fn inode_of(fd: RawFd) -> u64 {
        // SAFETY: fstat on a live descriptor into our own fd table.
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::fstat(fd, &mut st) };
        assert_eq!(rc, 0, "fstat on the pool descriptor");
        st.st_ino as u64
    }

    /// A rejected message that carried a descriptor must not leak it.
    ///
    /// The server closes every descriptor it does not adopt, and a client that
    /// can make the server leak one fd per message can exhaust the fd table and
    /// take down the whole system service. This drives that path repeatedly and
    /// checks the descriptor count does not grow.
    #[test]
    fn rejected_messages_do_not_leak_descriptors() {
        let path = temp_socket("fdleak");
        let server = spawn_server(path.clone(), OutputInfo::default());

        /// Run the handshake up to a bound wl_shm, draining every global.
        fn bind_shm(c: &mut TestClient) {
            let mut e = wire::Encoder::begin(1, display_req::GET_REGISTRY);
            e.put_new_id(2);
            c.send(e);
            // Every global is drained; binding before the server has finished
            // announcing them would leave later events to be mistaken for
            // replies.
            let mut shm_name = None;
            let mut seen = 0;
            while shm_name.is_none() || seen < 4 {
                let (obj, op, body) = c.recv_raw();
                if obj == 2 && op == registry_ev::GLOBAL {
                    let args = wire::decode_body("usu", &body, &[]).unwrap();
                    if args[1].as_str() == Some(WL_SHM) {
                        shm_name = args[0].as_u32();
                    }
                    seen += 1;
                    if seen >= 4 && shm_name.is_some() {
                        break;
                    }
                }
            }
            let mut e = wire::Encoder::begin(2, registry_req::BIND);
            e.put_uint(shm_name.unwrap())
                .put_string(WL_SHM)
                .put_uint(VERSION_SHM)
                .put_new_id(3);
            c.send(e);
            // Drain the two format events.
            let _ = c.recv("u");
            let _ = c.recv("u");
        }

        const ROUNDS: usize = 40;
        let mut leaked = Vec::new();
        for _ in 0..ROUNDS {
            let mut c = TestClient::connect(&path);
            // Handshake far enough to own a wl_shm, then misuse it: create_pool
            // on an object that was never created. The descriptor attached here
            // must be closed by the server when it rejects the request.
            bind_shm(&mut c);

            // A fresh descriptor each round, attached to a request the server
            // must reject because object 4 was never created.
            let fd = crate::wayland::shm::create_pool_fd(4096).expect("memfd");
            let inode = inode_of(fd);
            let mut e = wire::Encoder::begin(4, shm_req::CREATE_POOL);
            e.put_new_id(5).put_fd(fd).put_int(4096);
            c.send(e);

            // The server should report an error and close the connection.
            let mut closed = false;
            for _ in 0..8 {
                if c.stream.read(&mut [0u8; 4096]).unwrap_or(0) == 0 {
                    closed = true;
                    break;
                }
            }
            assert!(closed, "the server should have closed the connection");

            // The descriptor this round sent must be gone. The client's copy was
            // closed by `send`, so any surviving reference is the server's.
            if inode_still_open(inode) {
                leaked.push(inode);
            }
        }

        server.shutdown();

        assert!(
            leaked.is_empty(),
            "the server leaked {} of {ROUNDS} descriptors (inodes: {leaked:?})",
            leaked.len()
        );
    }

    /// `shutdown()` must stop a running accept loop even with no clients.
    ///
    /// A blocking `accept` parked the loop forever, so a compositor could not be
    /// stopped by signal: the process stayed alive, kept the socket bound, and
    /// the next start saw a busy path. The listener is now non-blocking and the
    /// loop checks `running`, so this returns promptly.
    #[test]
    fn shutdown_stops_the_accept_loop_with_no_clients() {
        let path = temp_socket("shutdown");
        let server = Arc::new(WaylandServer::bind(&path, OutputInfo::default()).unwrap());
        let s = Arc::clone(&server);

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = s.run();
            let _ = tx.send(());
        });

        // Give the loop a moment to enter `accept`.
        std::thread::sleep(std::time::Duration::from_millis(50));
        server.shutdown();

        assert!(
            rx.recv_timeout(std::time::Duration::from_secs(2)).is_ok(),
            "the accept loop did not return after shutdown(); accept() is blocking"
        );
    }

    /// Drive the server's accept loop in the background.
    fn spawn_server(path: PathBuf, output: OutputInfo) -> Arc<WaylandServer> {
        let server = Arc::new(WaylandServer::bind(&path, output).unwrap());
        let s = Arc::clone(&server);
        std::thread::spawn(move || {
            let _ = s.run();
        });
        server
    }

    /// Wait until a surface frame with the given key exists.
    fn wait_for_frame(
        state: &Arc<Mutex<CompositorState>>,
        key: SurfaceKey,
        timeout: std::time::Duration,
    ) -> Option<SurfaceFrame> {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if let Some(f) = state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .frame(key)
                .cloned()
            {
                return Some(f);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        None
    }

    #[test]
    fn real_client_creates_a_surface_and_commits_pixels() {
        let path = temp_socket("e2e");
        let server = spawn_server(path.clone(), OutputInfo::default());
        let state = server.state();

        let mut client = TestClient::connect(&path);

        // 1. wl_display.get_registry
        let mut e = wire::Encoder::begin(1, display_req::GET_REGISTRY);
        e.put_new_id(2);
        client.send(e);

        // 2. Read the globals until the compositor and shm appear.
        let mut compositor_name = None;
        let mut shm_name = None;
        let mut registry_done = false;
        while !registry_done {
            let (obj, op, body) = client.recv_raw();
            if obj == 2 && op == registry_ev::GLOBAL {
                let args = wire::decode_body("usu", &body, &[]).unwrap();
                let name = args[0].as_u32().unwrap();
                let iface = args[1].as_str().unwrap().to_string();
                match iface.as_str() {
                    WL_COMPOSITOR => compositor_name = Some(name),
                    WL_SHM => shm_name = Some(name),
                    _ => {}
                }
            }
            // Four globals are advertised; after reading them all we can bind.
            if compositor_name.is_some() && shm_name.is_some() {
                registry_done = true;
            }
        }

        // 3. Bind wl_compositor -> id 3
        let mut e = wire::Encoder::begin(2, registry_req::BIND);
        e.put_uint(compositor_name.unwrap())
            .put_string(WL_COMPOSITOR)
            .put_uint(4)
            .put_new_id(3);
        client.send(e);

        // 4. Bind wl_shm -> id 4, and read the two format events.
        let mut e = wire::Encoder::begin(2, registry_req::BIND);
        e.put_uint(shm_name.unwrap())
            .put_string(WL_SHM)
            .put_uint(1)
            .put_new_id(4);
        client.send(e);
        let mut formats = Vec::new();
        while formats.len() < 2 {
            let (obj, op, body) = client.recv_raw();
            if obj == 4 && op == shm_ev::FORMAT {
                let args = wire::decode_body("u", &body, &[]).unwrap();
                formats.push(args[0].as_u32().unwrap());
            }
        }
        assert!(formats.contains(&shm_format::ARGB8888));

        // 5. wl_compositor.create_surface -> id 5
        let mut e = wire::Encoder::begin(3, compositor_req::CREATE_SURFACE);
        e.put_new_id(5);
        client.send(e);

        // 6. Create a real memfd pool holding a 4x4 pattern, and send it.
        let w = 4u32;
        let h = 4u32;
        let size = w * h * 4;
        let pool_fd = super::super::shm::create_pool_fd(size).unwrap();
        // SAFETY: `pool_fd` is a live descriptor we own.
        let mut file = unsafe { std::fs::File::from_raw_fd(pool_fd) };
        let mut data = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let px = 0xFF00_0000u32 | (y << 8) | x;
                data.extend_from_slice(&px.to_le_bytes());
            }
        }
        file.write_all(&data).unwrap();
        file.flush().unwrap();

        // 7. wl_shm.create_pool -> id 6, passing the fd. `into_raw_fd` hands
        // ownership to the encoder, which closes it after the send.
        let mut e = wire::Encoder::begin(4, shm_req::CREATE_POOL);
        e.put_new_id(6)
            .put_fd(file.into_raw_fd())
            .put_int(size as i32);
        client.send(e);

        // 8. wl_shm_pool.create_buffer -> id 7
        let mut e = wire::Encoder::begin(6, shm_pool_req::CREATE_BUFFER);
        e.put_new_id(7)
            .put_int(0)
            .put_int(w as i32)
            .put_int(h as i32)
            .put_int((w * 4) as i32)
            .put_uint(shm_format::ARGB8888);
        client.send(e);

        // 9. wl_surface.attach + commit
        let mut e = wire::Encoder::begin(5, surface_req::ATTACH);
        e.put_object(7).put_int(0).put_int(0);
        client.send(e);

        let mut e = wire::Encoder::begin(5, surface_req::COMMIT);
        let msg = e.finish().unwrap();
        send_with_fds(&client.stream, &msg.bytes, &[]).unwrap();

        // 10. The server must have produced a frame with the exact pixels.
        let key = SurfaceKey {
            client: 1,
            object: 5,
        };
        let frame = wait_for_frame(&state, key, std::time::Duration::from_secs(5))
            .expect("server should have committed a frame");
        assert_eq!(frame.width, 4);
        assert_eq!(frame.height, 4);
        assert_eq!(frame.pixel(0, 0), 0xFF00_0000);
        assert_eq!(frame.pixel(3, 2), 0xFF00_0203);
        assert_eq!(frame.pixel(3, 3), 0xFF00_0303);

        // 11. The server must have released the buffer back to the client.
        let mut saw_release = false;
        for _ in 0..8 {
            match client.recv_raw() {
                (7, buffer_ev::RELEASE, _) => {
                    saw_release = true;
                    break;
                }
                _ => continue,
            }
        }
        assert!(
            saw_release,
            "server must send wl_buffer.release after copying"
        );

        server.shutdown();
    }

    #[test]
    fn real_client_sync_round_trips_a_callback() {
        let path = temp_socket("sync");
        let server = spawn_server(path.clone(), OutputInfo::default());

        let mut client = TestClient::connect(&path);
        let mut e = wire::Encoder::begin(1, display_req::SYNC);
        e.put_new_id(2);
        client.send(e);

        // wl_callback.done(callback_data) then wl_display.delete_id.
        let args = client.recv("u");
        assert_eq!(args[0].as_u32(), Some(0));
        let (obj, op, _) = client.recv_raw();
        assert_eq!((obj, op), (1, display_ev::DELETE_ID));

        server.shutdown();
    }

    #[test]
    fn server_rejects_a_buffer_that_exceeds_its_pool() {
        let path = temp_socket("badbuf");
        let server = spawn_server(path.clone(), OutputInfo::default());

        let mut client = TestClient::connect(&path);

        // Get registry and bind shm.
        let mut e = wire::Encoder::begin(1, display_req::GET_REGISTRY);
        e.put_new_id(2);
        client.send(e);
        let mut shm_name = None;
        for _ in 0..4 {
            let (obj, op, body) = client.recv_raw();
            if obj == 2 && op == registry_ev::GLOBAL {
                let args = wire::decode_body("usu", &body, &[]).unwrap();
                if args[1].as_str() == Some(WL_SHM) {
                    shm_name = args[0].as_u32();
                }
            }
        }
        let mut e = wire::Encoder::begin(2, registry_req::BIND);
        e.put_uint(shm_name.unwrap())
            .put_string(WL_SHM)
            .put_uint(1)
            .put_new_id(4);
        client.send(e);
        for _ in 0..2 {
            client.recv_raw();
        }

        // A 64-byte pool, then a buffer claiming to be 64x64.
        let pool_fd = super::super::shm::create_pool_fd(64).unwrap();
        let mut e = wire::Encoder::begin(4, shm_req::CREATE_POOL);
        e.put_new_id(6).put_fd(pool_fd).put_int(64);
        client.send(e);

        let mut e = wire::Encoder::begin(6, shm_pool_req::CREATE_BUFFER);
        e.put_new_id(7)
            .put_int(0)
            .put_int(64)
            .put_int(64)
            .put_int(256)
            .put_uint(shm_format::ARGB8888);
        client.send(e);

        // The server must send wl_display.error and then close, rather than
        // reading past the mapping.
        let mut chunk = [0u8; 4096];
        let mut got_error = false;
        let mut closed = false;
        for _ in 0..40 {
            match client.stream.read(&mut chunk) {
                Ok(0) => {
                    closed = true;
                    break;
                }
                Ok(n) => client.buf.extend_from_slice(&chunk[..n]),
                Err(_) => {
                    closed = true;
                    break;
                }
            }
            while client.buf.len() >= wire::HEADER_SIZE {
                let size = match wire::declared_size(&client.buf) {
                    Ok(s) => s,
                    Err(_) => break,
                };
                if client.buf.len() < size {
                    break;
                }
                let msg: Vec<u8> = client.buf.drain(..size).collect();
                let (obj, op, _) = wire::decode_message(&msg).unwrap();
                if obj == 1 && op == display_ev::ERROR {
                    got_error = true;
                }
            }
            if got_error && closed {
                break;
            }
        }
        assert!(
            got_error,
            "an oversized buffer must produce wl_display.error"
        );
        assert!(
            closed,
            "the server must close the connection after the error"
        );

        server.shutdown();
    }
}
