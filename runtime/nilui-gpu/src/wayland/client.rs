// runtime/nilui-gpu/src/wayland/client.rs — a Wayland client for surface producers
//
// This is the *producer* side of the compositor: the code a component runs when
// it has pixels to show. It is the piece an Android graphics bridge needs,
// because Android's SurfaceFlinger hands over buffers and something has to push
// them at a compositor.
//
// Scope, stated plainly because it is the difference between a working bridge
// and a claim that cannot be backed:
//
//   * What this does: connect, negotiate `wl_compositor` + `wl_shm`, create a
//     surface, create a shared-memory pool, and commit frames from a CPU pixel
//     buffer. Every step is exercised against the real server in this crate's
//     tests and in `tests/wayland_cross_process.rs`.
//
//   * What this does *not* do: accept a GPU/dma-buf handle from Android. A
//     caller must have the pixels in CPU memory. Copying a SurfaceFlinger
//     gralloc buffer into a shm pool is the caller's job, and on a real device
//     that copy is a per-frame cost worth eliminating later with dmabuf.
//
// The protocol is spoken directly rather than through libwayland, matching the
// server side; see `wire.rs` for why.

use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use super::protocol::*;
use super::shm::{create_pool_fd, ShmPool};
use super::wire::{self, Encoder};

/// Errors a producer can hit. Each names the step that failed, because a
/// graphics bridge that silently stops presenting is hard to debug otherwise.
#[derive(Debug)]
pub enum ClientError {
    /// The compositor socket could not be reached.
    Connect {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The compositor did not advertise an interface we require.
    MissingGlobal(&'static str),
    /// A message could not be encoded.
    Encode(String),
    /// A message could not be decoded.
    Decode(String),
    /// The compositor closed the connection.
    Disconnected,
    /// A transport failure.
    Io(std::io::Error),
    /// The shm pool could not be created or mapped.
    Pool(String),
    /// `surface_mut` was called before the pool existed, which cannot happen
    /// through the public API but would be a logic error if it did.
    NotReady,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Connect { path, source } => {
                write!(f, "could not connect to {}: {source}", path.display())
            }
            ClientError::MissingGlobal(name) => {
                write!(f, "the compositor does not offer {name}")
            }
            ClientError::Encode(e) => write!(f, "could not encode a request: {e}"),
            ClientError::Decode(e) => write!(f, "could not decode a reply: {e}"),
            ClientError::Disconnected => write!(f, "the compositor closed the connection"),
            ClientError::Io(e) => write!(f, "transport failure: {e}"),
            ClientError::Pool(e) => write!(f, "shared-memory pool failure: {e}"),
            ClientError::NotReady => write!(f, "the surface was used before it was created"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> Self {
        ClientError::Io(e)
    }
}

/// A connected Wayland client that owns one surface.
///
/// The surface's pixels live in a shared-memory pool this type allocates. Write
/// them with [`SurfaceProducer::frame_mut`], then call
/// [`SurfaceProducer::commit`] to hand the frame to the compositor.
pub struct SurfaceProducer {
    stream: UnixStream,
    read_buf: Vec<u8>,
    next_id: u32,
    display: u32,
    registry: u32,
    compositor: u32,
    shm: u32,
    surface: u32,
    pool: u32,
    buffer: u32,
    /// Width/height of the frame, in pixels.
    width: u32,
    height: u32,
    /// The mapping the caller writes frames through.
    frame: Vec<u32>,
    /// The descriptor used to publish `frame` into the shared object.
    publish_fd: std::fs::File,
    /// Set once the buffer has been attached at least once.
    attached: bool,
}

/// Debug shows the negotiated identity, not the buffers, so a test failure
/// prints something readable rather than a megabyte of pixels.
impl std::fmt::Debug for SurfaceProducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurfaceProducer")
            .field("surface", &self.surface)
            .field("buffer", &self.buffer)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("attached", &self.attached)
            .finish()
    }
}

impl SurfaceProducer {
    /// Connect to a compositor and set up a surface of `width` x `height`.
    ///
    /// `path` is the compositor's Unix socket, normally
    /// `$XDG_RUNTIME_DIR/wayland-0`.
    pub fn connect(path: &Path, width: u32, height: u32) -> Result<Self, ClientError> {
        if width == 0 || height == 0 {
            return Err(ClientError::Pool("frame size must be non-zero".into()));
        }
        let stream = UnixStream::connect(path).map_err(|e| ClientError::Connect {
            path: path.to_path_buf(),
            source: e,
        })?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;

        let mut c = Self {
            stream,
            read_buf: Vec::new(),
            next_id: 2, // object 1 is wl_display
            display: 1,
            registry: 0,
            compositor: 0,
            shm: 0,
            surface: 0,
            pool: 0,
            buffer: 0,
            width,
            height,
            frame: vec![0u32; (width as usize) * (height as usize)],
            publish_fd: {
                // Placeholder replaced below; a File cannot be defaulted.
                let fd = create_pool_fd(4).map_err(|e| ClientError::Pool(e.to_string()))?;
                // SAFETY: `fd` is a live descriptor we own.
                unsafe { std::fs::File::from_raw_fd(fd) }
            },
            attached: false,
        };

        c.handshake()?;
        c.create_pool()?;
        Ok(c)
    }

    /// The default socket path, honouring `XDG_RUNTIME_DIR`.
    pub fn default_socket_path() -> PathBuf {
        super::server::default_socket_path()
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Send one message, closing any descriptors it carried.
    fn send(&mut self, mut enc: Encoder) -> Result<(), ClientError> {
        let msg = enc
            .finish()
            .map_err(|e| ClientError::Encode(e.to_string()))?;
        super::server::send_with_fds(&self.stream, &msg.bytes, &msg.fds)?;
        for fd in msg.fds {
            // SAFETY: the kernel duplicated each descriptor into the compositor.
            unsafe { libc::close(fd) };
        }
        Ok(())
    }

    /// Read one message.
    fn recv_raw(&mut self) -> Result<(u32, u16, Vec<u8>), ClientError> {
        loop {
            if self.read_buf.len() >= wire::HEADER_SIZE {
                let size = wire::declared_size(&self.read_buf)
                    .map_err(|e| ClientError::Decode(e.to_string()))?;
                if self.read_buf.len() >= size {
                    let msg: Vec<u8> = self.read_buf.drain(..size).collect();
                    let (obj, op, body) = wire::decode_message(&msg)
                        .map_err(|e| ClientError::Decode(e.to_string()))?;
                    return Ok((obj, op, body.to_vec()));
                }
            }
            let mut chunk = [0u8; 8192];
            let n = self.stream.read(&mut chunk)?;
            if n == 0 {
                return Err(ClientError::Disconnected);
            }
            self.read_buf.extend_from_slice(&chunk[..n]);
        }
    }

    /// Connect the registry, bind the globals a surface needs, and create it.
    fn handshake(&mut self) -> Result<(), ClientError> {
        let registry = self.alloc_id();
        self.registry = registry;
        let mut e = Encoder::begin(self.display, display_req::GET_REGISTRY);
        e.put_new_id(registry);
        self.send(e)?;

        // Collect globals until both required ones are seen. The compositor may
        // send more later; that is fine, they are simply ignored.
        let mut compositor_name = None;
        let mut shm_name = None;
        while compositor_name.is_none() || shm_name.is_none() {
            let (obj, op, body) = self.recv_raw()?;
            if obj == registry && op == registry_ev::GLOBAL {
                let args = wire::decode_body("usu", &body, &[])
                    .map_err(|e| ClientError::Decode(e.to_string()))?;
                let name = args[0].as_u32().ok_or(ClientError::NotReady)?;
                match args[1].as_str().unwrap_or("") {
                    WL_COMPOSITOR => compositor_name = Some(name),
                    WL_SHM => shm_name = Some(name),
                    _ => {}
                }
            }
        }

        let compositor = self.alloc_id();
        self.compositor = compositor;
        let mut e = Encoder::begin(registry, registry_req::BIND);
        e.put_uint(compositor_name.ok_or(ClientError::MissingGlobal(WL_COMPOSITOR))?)
            .put_string(WL_COMPOSITOR)
            .put_uint(VERSION_COMPOSITOR)
            .put_new_id(compositor);
        self.send(e)?;

        let shm = self.alloc_id();
        self.shm = shm;
        let mut e = Encoder::begin(registry, registry_req::BIND);
        e.put_uint(shm_name.ok_or(ClientError::MissingGlobal(WL_SHM))?)
            .put_string(WL_SHM)
            .put_uint(VERSION_SHM)
            .put_new_id(shm);
        self.send(e)?;

        // Drain the wl_shm.format events; we only need to know the stream is in
        // step, and that ARGB8888 was offered.
        let mut formats = Vec::new();
        while formats.len() < 2 {
            let (obj, op, body) = self.recv_raw()?;
            if obj == shm && op == shm_ev::FORMAT {
                let args = wire::decode_body("u", &body, &[])
                    .map_err(|e| ClientError::Decode(e.to_string()))?;
                if let Some(f) = args[0].as_u32() {
                    formats.push(f);
                }
            }
        }
        if !formats.contains(&shm_format::ARGB8888) {
            return Err(ClientError::MissingGlobal("wl_shm ARGB8888"));
        }

        let surface = self.alloc_id();
        self.surface = surface;
        let mut e = Encoder::begin(compositor, compositor_req::CREATE_SURFACE);
        e.put_new_id(surface);
        self.send(e)?;
        Ok(())
    }

    /// Allocate the pool and the single buffer this producer reuses.
    fn create_pool(&mut self) -> Result<(), ClientError> {
        let stride = self.width * 4;
        let size = stride * self.height;
        let fd = create_pool_fd(size).map_err(|e| ClientError::Pool(e.to_string()))?;

        // One descriptor goes to the compositor; a duplicate stays here so
        // frames can be written into the same object.
        let local_fd = unsafe { libc::dup(fd) };
        if local_fd < 0 {
            unsafe { libc::close(fd) };
            return Err(ClientError::Pool(
                "dup of the pool descriptor failed".into(),
            ));
        }

        let pool = self.alloc_id();
        self.pool = pool;
        let mut e = Encoder::begin(self.shm, shm_req::CREATE_POOL);
        e.put_new_id(pool).put_fd(fd).put_int(size as i32);
        self.send(e)?;

        let buffer = self.alloc_id();
        self.buffer = buffer;
        let mut e = Encoder::begin(pool, shm_pool_req::CREATE_BUFFER);
        e.put_new_id(buffer)
            .put_int(0)
            .put_int(self.width as i32)
            .put_int(self.height as i32)
            .put_int(stride as i32)
            .put_uint(shm_format::ARGB8888);
        self.send(e)?;

        // SAFETY: `local_fd` is a live descriptor we own.
        self.publish_fd = unsafe { std::fs::File::from_raw_fd(local_fd) };
        Ok(())
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The frame buffer to draw into.
    ///
    /// Format is packed ARGB8888 (`0xAARRGGBB`), row-major and tightly packed.
    /// An alpha of `0xFF` is opaque; anything less blends over what is below.
    pub fn frame_mut(&mut self) -> &mut [u32] {
        &mut self.frame
    }

    /// Fill the whole frame with one colour.
    pub fn fill(&mut self, argb: u32) {
        self.frame.fill(argb);
    }

    /// Hand the current frame to the compositor and wait for it to be copied.
    ///
    /// Waiting for `wl_buffer.release` is what makes reusing one buffer safe: it
    /// guarantees the compositor has finished reading before the caller writes
    /// the next frame.
    pub fn commit(&mut self) -> Result<(), ClientError> {
        // Publish the pixels into the shared object.
        {
            use std::io::Seek;
            let mut bytes = Vec::with_capacity(self.frame.len() * 4);
            for px in &self.frame {
                bytes.extend_from_slice(&px.to_le_bytes());
            }
            self.publish_fd.seek(std::io::SeekFrom::Start(0))?;
            self.publish_fd.write_all(&bytes)?;
            self.publish_fd.flush()?;
        }

        // Attach once, then re-commit the same buffer for later frames.
        if !self.attached {
            let mut e = Encoder::begin(self.surface, surface_req::ATTACH);
            e.put_object(self.buffer).put_int(0).put_int(0);
            self.send(e)?;
            self.attached = true;
        }

        self.send(Encoder::begin(self.surface, surface_req::COMMIT))?;

        // Wait for the release, ignoring any other events in between.
        loop {
            let (obj, op, _) = self.recv_raw()?;
            if obj == self.buffer && op == buffer_ev::RELEASE {
                return Ok(());
            }
            // A protocol error ends the session.
            if obj == self.display && op == display_ev::ERROR {
                return Err(ClientError::Decode(
                    "the compositor reported a protocol error".into(),
                ));
            }
        }
    }

    /// Map this producer's pool for reading, for a self-check.
    ///
    /// Used by tests to confirm what the compositor would see; a normal
    /// producer has no reason to read its own frames back.
    pub fn read_back(&self) -> Result<ShmPool, ClientError> {
        let fd = self.publish_fd.as_raw_fd();
        // A duplicate is mapped so closing it cannot disturb the publish path.
        let dup = unsafe { libc::dup(fd) };
        if dup < 0 {
            return Err(ClientError::Pool("dup for read-back failed".into()));
        }
        ShmPool::map(dup, self.width * 4 * self.height)
            .map_err(|e| ClientError::Pool(e.to_string()))
    }

    /// Destroy the surface and pool, then drop the connection.
    pub fn close(mut self) -> Result<(), ClientError> {
        self.send(Encoder::begin(self.surface, surface_req::DESTROY))?;
        self.send(Encoder::begin(self.buffer, buffer_req::DESTROY))?;
        self.send(Encoder::begin(self.pool, shm_pool_req::DESTROY))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::compose::{composite, Layout};
    use crate::wayland::server::{OutputInfo, SurfaceKey, WaylandServer};
    use std::sync::Arc;

    fn temp_socket(tag: &str) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "onuron-wl-client-{tag}-{}-{n}.sock",
            std::process::id()
        ))
    }

    fn spawn_server(path: PathBuf, output: OutputInfo) -> Arc<WaylandServer> {
        let server = Arc::new(WaylandServer::bind(&path, output).unwrap());
        let s = Arc::clone(&server);
        std::thread::spawn(move || {
            let _ = s.run();
        });
        server
    }

    /// Wait for a frame, then return it.
    fn await_frame(
        state: &Arc<std::sync::Mutex<crate::wayland::server::CompositorState>>,
        key: SurfaceKey,
    ) -> crate::wayland::server::SurfaceFrame {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if let Some(f) = state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .frame(key)
                .cloned()
            {
                return f;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("the compositor never committed a frame for {key:?}");
    }

    #[test]
    fn producer_pushes_exact_pixels_to_the_compositor() {
        let path = temp_socket("push");
        let server = spawn_server(
            path.clone(),
            OutputInfo {
                width: 32,
                height: 32,
                refresh_mhz: 60_000,
            },
        );
        let state = server.state();

        let mut p = SurfaceProducer::connect(&path, 4, 4).expect("connect");
        assert_eq!(p.width(), 4);
        assert_eq!(p.height(), 4);

        // Draw a known pattern through the public API.
        for y in 0..4u32 {
            for x in 0..4u32 {
                p.frame_mut()[(y * 4 + x) as usize] = 0xFF00_0000 | (y << 8) | x;
            }
        }
        p.commit().expect("commit");

        // The compositor must have the exact pixels.
        let frame = await_frame(
            &state,
            SurfaceKey {
                client: 1,
                object: 5,
            },
        );
        assert_eq!(frame.width, 4);
        assert_eq!(frame.height, 4);
        assert_eq!(frame.pixel(0, 0), 0xFF00_0000);
        assert_eq!(frame.pixel(3, 2), 0xFF00_0203);
        assert_eq!(frame.pixel(3, 3), 0xFF00_0303);

        // And the read-back path must agree with what was written.
        let pool = p.read_back().expect("read back");
        let layout = pool.validate_buffer(0, 4, 4, 16).unwrap();
        assert_eq!(pool.pixel(&layout, 3, 2), 0xFF00_0203);

        p.close().expect("close");
        server.shutdown();
    }

    #[test]
    fn a_second_commit_replaces_the_first() {
        let path = temp_socket("twice");
        let server = spawn_server(path.clone(), OutputInfo::default());
        let state = server.state();

        let mut p = SurfaceProducer::connect(&path, 2, 2).expect("connect");
        p.fill(0xFF111111);
        p.commit().expect("first commit");
        let first = await_frame(
            &state,
            SurfaceKey {
                client: 1,
                object: 5,
            },
        );
        assert_eq!(first.pixel(0, 0), 0xFF111111);

        // Change the frame and commit again through the same buffer.
        p.fill(0xFF222222);
        p.commit().expect("second commit");

        // The surface must now hold the new colour.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut latest = first;
        while std::time::Instant::now() < deadline {
            let f = state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .frame(SurfaceKey {
                    client: 1,
                    object: 5,
                })
                .cloned()
                .unwrap();
            if f.pixel(0, 0) == 0xFF222222 {
                latest = f;
                break;
            }
            latest = f;
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(
            latest.pixel(0, 0),
            0xFF222222,
            "a second commit must replace the surface contents"
        );

        p.close().expect("close");
        server.shutdown();
    }

    #[test]
    fn committed_pixels_survive_compositing_into_a_frame() {
        let path = temp_socket("compose");
        let server = spawn_server(path.clone(), OutputInfo::default());
        let state = server.state();

        let mut p = SurfaceProducer::connect(&path, 2, 2).expect("connect");
        p.fill(0xFF3366CC);
        p.commit().expect("commit");
        let _ = await_frame(
            &state,
            SurfaceKey {
                client: 1,
                object: 5,
            },
        );

        // Composite into a real output buffer, which is what the compositor
        // binary does before presenting.
        let mut dst = crate::compositor::PixelBuffer::new(64, 64);
        dst.clear(0xFF000000);
        let drawn = {
            let st = state.lock().unwrap_or_else(|e| e.into_inner());
            composite(&mut dst, &st, Layout::Fullscreen)
        };
        assert_eq!(drawn, 1);
        // The 2x2 surface is scaled up to fill, so the centre is its colour.
        assert_eq!(dst.get_pixel(32, 32), 0xFF3366CC);

        p.close().expect("close");
        server.shutdown();
    }

    #[test]
    fn connect_to_a_missing_socket_is_a_clear_error() {
        let missing = temp_socket("absent");
        let err = SurfaceProducer::connect(&missing, 8, 8).unwrap_err();
        match err {
            ClientError::Connect { path, .. } => assert_eq!(path, missing),
            other => panic!("expected a Connect error, got {other:?}"),
        }
    }

    #[test]
    fn a_zero_sized_surface_is_refused() {
        let path = temp_socket("zero");
        let err = SurfaceProducer::connect(&path, 0, 8).unwrap_err();
        assert!(matches!(err, ClientError::Pool(_)), "got {err:?}");
    }
}
