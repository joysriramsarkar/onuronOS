// runtime/nilui-gpu/src/wayland/wire.rs — Wayland wire protocol codec
//
// The Wayland wire format is a stream of messages over a Unix socket:
//
//     u32 object_id | u16 size | u16 opcode | <arguments...>
//
// `size` covers the 8-byte header and must be a multiple of 4. Every argument
// is padded to a 4-byte boundary. This module implements that format directly
// rather than depending on libwayland, for the same reason `drm.rs` hand-rolls
// its ioctls: the image has no `libwayland-server` headers, the dependency
// policy in `deny.toml` gates unmaintained crates, and the musl release build
// statically links, which a C library dependency would complicate.
//
// Two details are easy to get wrong and are pinned by tests below:
//
//   * Strings and arrays are length-prefixed *including* the NUL terminator for
//     strings, and the padding is computed from the *unpadded* length.
//   * An `fd` argument occupies **zero** bytes in the message body. The
//     descriptor travels as `SCM_RIGHTS` ancillary data; the signature entry
//     only tells the receiver to consume the next fd from that array. A codec
//     that wrote 4 bytes for `h` would desynchronise every following argument.

use std::os::fd::RawFd;

/// Fixed part of every message.
pub const HEADER_SIZE: usize = 8;

/// Upper bound on a single message. Wayland's own default receive buffer is
/// 4096 bytes; we allow more for larger `wl_shm`-adjacent requests but still
/// refuse anything absurd so a bad peer cannot make us allocate unboundedly.
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// The buffer ended before every declared argument was read.
    Truncated { needed: usize, available: usize },
    /// `size` was smaller than the header, not a multiple of 4, or too large.
    BadSize(u16),
    /// A string or array length ran past the end of the message.
    BadLength { declared: usize, available: usize },
    /// A string was not NUL-terminated, or its declared length was 0.
    BadString,
    /// The body had trailing bytes the signature did not account for.
    TrailingBytes(usize),
    /// The peer declared more fds than it actually sent.
    MissingFd,
    /// An argument value was out of range for its type.
    OutOfRange(&'static str),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::Truncated { needed, available } => {
                write!(
                    f,
                    "truncated message: needed {needed} bytes, {available} available"
                )
            }
            WireError::BadSize(s) => write!(f, "invalid message size {s}"),
            WireError::BadLength {
                declared,
                available,
            } => write!(
                f,
                "string/array length {declared} exceeds the {available} bytes remaining"
            ),
            WireError::BadString => write!(f, "string is not NUL-terminated"),
            WireError::TrailingBytes(n) => write!(f, "{n} trailing bytes after the arguments"),
            WireError::MissingFd => write!(f, "message references an fd the peer did not send"),
            WireError::OutOfRange(what) => write!(f, "{what} is out of range"),
        }
    }
}

impl std::error::Error for WireError {}

/// Round `n` up to the next multiple of 4, as the wire format requires.
#[inline]
pub const fn pad_len(n: usize) -> usize {
    (4 - (n % 4)) % 4
}

/// One decoded argument.
///
/// `Fixed` is the 24.8 signed fixed-point type Wayland uses for coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Int(i32),
    Uint(u32),
    Fixed(f64),
    Str(String),
    Array(Vec<u8>),
    Object(u32),
    NewId(u32),
    /// An fd argument. The descriptor is delivered out of band via SCM_RIGHTS;
    /// this holds the descriptor the receiver extracted.
    Fd(RawFd),
}

impl Arg {
    /// The integer payload, for callers that know the concrete type.
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Arg::Uint(v) | Arg::Object(v) | Arg::NewId(v) => Some(*v),
            Arg::Int(v) => Some(*v as u32),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Arg::Int(v) => Some(*v),
            Arg::Uint(v) | Arg::Object(v) | Arg::NewId(v) => Some(*v as i32),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Arg::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[u8]> {
        match self {
            Arg::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_fd(&self) -> Option<RawFd> {
        match self {
            Arg::Fd(fd) => Some(*fd),
            _ => None,
        }
    }

    /// The 24.8 fixed-point value as a float.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Arg::Fixed(v) => Some(*v),
            _ => None,
        }
    }
}

/// Convert a float to Wayland's 24.8 signed fixed-point representation.
pub fn f64_to_fixed(v: f64) -> i32 {
    // 24.8 means 24 bits of integer part and 8 bits of fraction. Clamping keeps
    // a hostile or buggy caller from wrapping the sign.
    let scaled = (v * 256.0).round();
    scaled.clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

/// Convert a 24.8 fixed-point value back to a float.
pub fn fixed_to_f64(v: i32) -> f64 {
    f64::from(v) / 256.0
}

/// Builds a Wayland message body.
#[derive(Debug, Default)]
pub struct Encoder {
    buf: Vec<u8>,
    fds: Vec<RawFd>,
}

impl Encoder {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(64),
            fds: Vec::new(),
        }
    }

    /// Start a message. The header size is patched once the body is complete.
    pub fn begin(object_id: u32, opcode: u16) -> Self {
        let mut e = Self::new();
        e.buf.extend_from_slice(&object_id.to_ne_bytes());
        e.buf.extend_from_slice(&0u16.to_ne_bytes()); // size placeholder
        e.buf.extend_from_slice(&opcode.to_ne_bytes());
        e
    }

    fn pad(&mut self) {
        let p = pad_len(self.buf.len() - HEADER_SIZE);
        // Padding bytes are unspecified by the protocol; zero is conventional.
        self.buf.extend(std::iter::repeat_n(0u8, p));
    }

    pub fn put_int(&mut self, v: i32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_ne_bytes());
        self
    }

    pub fn put_uint(&mut self, v: u32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_ne_bytes());
        self
    }

    pub fn put_object(&mut self, id: u32) -> &mut Self {
        self.put_uint(id)
    }

    pub fn put_new_id(&mut self, id: u32) -> &mut Self {
        self.put_uint(id)
    }

    pub fn put_fixed(&mut self, v: f64) -> &mut Self {
        self.buf.extend_from_slice(&f64_to_fixed(v).to_ne_bytes());
        self
    }

    pub fn put_string(&mut self, s: &str) -> &mut Self {
        // Length includes the NUL terminator, and the terminator counts toward
        // the padded length even though it is not part of `s`.
        let len = s.len() + 1;
        self.buf.extend_from_slice(&(len as u32).to_ne_bytes());
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.push(0);
        self.pad();
        self
    }

    /// Write a string argument that is absent, which the protocol encodes as
    /// length 0 and no payload.
    pub fn put_null_string(&mut self) -> &mut Self {
        self.buf.extend_from_slice(&0u32.to_ne_bytes());
        self
    }

    pub fn put_array(&mut self, data: &[u8]) -> &mut Self {
        self.buf
            .extend_from_slice(&(data.len() as u32).to_ne_bytes());
        self.buf.extend_from_slice(data);
        self.pad();
        self
    }

    /// Record an fd argument. It adds no bytes to the body; it is queued for
    /// SCM_RIGHTS transmission instead.
    ///
    /// **Ownership:** the encoder takes ownership of `fd`. Whoever sends the
    /// finished message is responsible for closing it once the kernel has
    /// duplicated it into the peer's descriptor table (see
    /// `server::send_with_fds`). Callers that still hold an owning handle must
    /// hand over `into_raw_fd()`, not `as_raw_fd()`, or the descriptor is
    /// closed twice.
    pub fn put_fd(&mut self, fd: RawFd) -> &mut Self {
        self.fds.push(fd);
        self
    }

    /// Finish the message, patching the size field. Errors if the message would
    /// exceed [`MAX_MESSAGE_SIZE`].
    ///
    /// Takes `&mut self` rather than `self` so a caller can chain straight from
    /// a temporary (`Encoder::begin(..).put_uint(..).finish()`); the buffers are
    /// moved out, leaving the encoder empty.
    pub fn finish(&mut self) -> Result<EncodedMessage, WireError> {
        let size = self.buf.len();
        if size > MAX_MESSAGE_SIZE || size > u16::MAX as usize {
            return Err(WireError::BadSize(size.min(u16::MAX as usize) as u16));
        }
        self.buf[4..6].copy_from_slice(&(size as u16).to_ne_bytes());
        Ok(EncodedMessage {
            bytes: std::mem::take(&mut self.buf),
            fds: std::mem::take(&mut self.fds),
        })
    }
}

/// A serialised message plus any descriptors to send alongside it.
#[derive(Debug, Clone)]
pub struct EncodedMessage {
    pub bytes: Vec<u8>,
    pub fds: Vec<RawFd>,
}

impl EncodedMessage {
    pub fn object_id(&self) -> u32 {
        u32::from_ne_bytes([self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3]])
    }

    pub fn opcode(&self) -> u16 {
        u16::from_ne_bytes([self.bytes[6], self.bytes[7]])
    }
}

/// A decoded message.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedMessage {
    pub object_id: u32,
    pub opcode: u16,
    pub args: Vec<Arg>,
}

impl DecodedMessage {
    pub fn arg(&self, index: usize) -> Option<&Arg> {
        self.args.get(index)
    }

    pub fn u32_arg(&self, index: usize) -> Option<u32> {
        self.args.get(index).and_then(Arg::as_u32)
    }

    pub fn i32_arg(&self, index: usize) -> Option<i32> {
        self.args.get(index).and_then(Arg::as_i32)
    }

    pub fn str_arg(&self, index: usize) -> Option<&str> {
        self.args.get(index).and_then(Arg::as_str)
    }

    pub fn fd_arg(&self, index: usize) -> Option<RawFd> {
        self.args.get(index).and_then(Arg::as_fd)
    }
}

/// Decodes one message from `body` (which excludes the 8-byte header) against a
/// protocol signature such as `"uso"`.
///
/// `fds` supplies the descriptors the kernel delivered as ancillary data, in
/// order. Each `h` in the signature consumes the next one.
pub fn decode_body(signature: &str, body: &[u8], fds: &[RawFd]) -> Result<Vec<Arg>, WireError> {
    let mut args = Vec::with_capacity(signature.len());
    let mut pos = 0usize;
    let mut fd_index = 0usize;

    let take = |pos: &mut usize, n: usize| -> Result<&[u8], WireError> {
        if *pos + n > body.len() {
            return Err(WireError::Truncated {
                needed: *pos + n,
                available: body.len(),
            });
        }
        let slice = &body[*pos..*pos + n];
        *pos += n;
        Ok(slice)
    };

    for ch in signature.chars() {
        match ch {
            'i' => {
                let b = take(&mut pos, 4)?;
                args.push(Arg::Int(i32::from_ne_bytes(b.try_into().unwrap())));
            }
            'u' | 'o' | 'n' => {
                let b = take(&mut pos, 4)?;
                let v = u32::from_ne_bytes(b.try_into().unwrap());
                args.push(match ch {
                    'u' => Arg::Uint(v),
                    'o' => Arg::Object(v),
                    _ => Arg::NewId(v),
                });
            }
            'f' => {
                let b = take(&mut pos, 4)?;
                let raw = i32::from_ne_bytes(b.try_into().unwrap());
                args.push(Arg::Fixed(fixed_to_f64(raw)));
            }
            's' => {
                let b = take(&mut pos, 4)?;
                let len = u32::from_ne_bytes(b.try_into().unwrap()) as usize;
                if len == 0 {
                    // Absent string.
                    args.push(Arg::Str(String::new()));
                    continue;
                }
                if len > body.len().saturating_sub(pos) {
                    return Err(WireError::BadLength {
                        declared: len,
                        available: body.len().saturating_sub(pos),
                    });
                }
                let bytes = take(&mut pos, len)?;
                if bytes[len - 1] != 0 {
                    return Err(WireError::BadString);
                }
                let s = String::from_utf8_lossy(&bytes[..len - 1]).into_owned();
                let p = pad_len(len);
                take(&mut pos, p)?;
                args.push(Arg::Str(s));
            }
            'a' => {
                let b = take(&mut pos, 4)?;
                let len = u32::from_ne_bytes(b.try_into().unwrap()) as usize;
                if len > body.len().saturating_sub(pos) {
                    return Err(WireError::BadLength {
                        declared: len,
                        available: body.len().saturating_sub(pos),
                    });
                }
                let bytes = take(&mut pos, len)?.to_vec();
                let p = pad_len(len);
                take(&mut pos, p)?;
                args.push(Arg::Array(bytes));
            }
            'h' => {
                // Zero body bytes: the descriptor came in via SCM_RIGHTS.
                let fd = *fds.get(fd_index).ok_or(WireError::MissingFd)?;
                fd_index += 1;
                args.push(Arg::Fd(fd));
            }
            _ => return Err(WireError::OutOfRange("unsupported argument type")),
        }
    }

    if pos != body.len() {
        return Err(WireError::TrailingBytes(body.len() - pos));
    }
    Ok(args)
}

/// Split a full message (header included) into its parts.
pub fn decode_message(bytes: &[u8]) -> Result<(u32, u16, &[u8]), WireError> {
    if bytes.len() < HEADER_SIZE {
        return Err(WireError::Truncated {
            needed: HEADER_SIZE,
            available: bytes.len(),
        });
    }
    let object_id = u32::from_ne_bytes(bytes[0..4].try_into().unwrap());
    let size = u16::from_ne_bytes(bytes[4..6].try_into().unwrap());
    let opcode = u16::from_ne_bytes(bytes[6..8].try_into().unwrap());
    if (size as usize) < HEADER_SIZE || size % 4 != 0 || size as usize > MAX_MESSAGE_SIZE {
        return Err(WireError::BadSize(size));
    }
    if (size as usize) > bytes.len() {
        return Err(WireError::Truncated {
            needed: size as usize,
            available: bytes.len(),
        });
    }
    Ok((object_id, opcode, &bytes[HEADER_SIZE..size as usize]))
}

/// The size a message declares in its header, for stream framing.
pub fn declared_size(bytes: &[u8]) -> Result<usize, WireError> {
    if bytes.len() < HEADER_SIZE {
        return Err(WireError::Truncated {
            needed: HEADER_SIZE,
            available: bytes.len(),
        });
    }
    let size = u16::from_ne_bytes(bytes[4..6].try_into().unwrap());
    if (size as usize) < HEADER_SIZE || size % 4 != 0 || size as usize > MAX_MESSAGE_SIZE {
        return Err(WireError::BadSize(size));
    }
    Ok(size as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_len_rounds_to_four() {
        assert_eq!(pad_len(0), 0);
        assert_eq!(pad_len(1), 3);
        assert_eq!(pad_len(2), 2);
        assert_eq!(pad_len(3), 1);
        assert_eq!(pad_len(4), 0);
        assert_eq!(pad_len(5), 3);
    }

    #[test]
    fn fixed_point_round_trips() {
        for v in [0.0, 1.0, -1.0, 0.5, 123.25, -0.00390625] {
            let back = fixed_to_f64(f64_to_fixed(v));
            assert!((back - v).abs() < 0.004, "{v} -> {back}");
        }
    }

    #[test]
    fn uint_message_matches_hand_computed_bytes() {
        // wl_registry.bind(1, "wl_compositor", 4, new_id=2)
        let msg = Encoder::begin(1, 0)
            .put_uint(1)
            .put_string("wl_compositor")
            .put_uint(4)
            .put_new_id(2)
            .finish()
            .unwrap();

        let expected: Vec<u8> = vec![
            0x01, 0x00, 0x00, 0x00, // object_id = 1
            0x28, 0x00, // size = 40
            0x00, 0x00, // opcode = 0
            0x01, 0x00, 0x00, 0x00, // name = 1
            0x0e, 0x00, 0x00, 0x00, // string length = 14 ("wl_compositor" + NUL)
            b'w', b'l', b'_', b'c', b'o', b'm', b'p', b'o', b's', b'i', b't', b'o', b'r', 0x00,
            0x00, 0x00, // padding: 14 % 4 == 2, so 2 bytes
            0x04, 0x00, 0x00, 0x00, // version = 4
            0x02, 0x00, 0x00, 0x00, // new_id = 2
        ];
        assert_eq!(msg.bytes, expected);
        assert_eq!(msg.bytes.len(), 40);
    }

    #[test]
    fn string_padding_is_computed_from_the_unpadded_length() {
        let m = Encoder::begin(1, 0).put_string("abc").finish().unwrap();
        assert_eq!(m.bytes.len(), HEADER_SIZE + 4 + 4);

        let m = Encoder::begin(1, 0).put_string("ab").finish().unwrap();
        assert_eq!(m.bytes.len(), HEADER_SIZE + 4 + 4);
        assert_eq!(*m.bytes.last().unwrap(), 0);

        let m = Encoder::begin(1, 0).put_string("abcd").finish().unwrap();
        assert_eq!(m.bytes.len(), HEADER_SIZE + 4 + 4 + 4);
    }

    #[test]
    fn empty_string_is_length_one_not_zero() {
        let m = Encoder::begin(1, 0).put_string("").finish().unwrap();
        assert_eq!(m.bytes.len(), HEADER_SIZE + 4 + 4);
        let body = &m.bytes[HEADER_SIZE..];
        assert_eq!(u32::from_ne_bytes(body[0..4].try_into().unwrap()), 1);
    }

    #[test]
    fn null_string_is_length_zero() {
        let m = Encoder::begin(1, 0).put_null_string().finish().unwrap();
        assert_eq!(m.bytes.len(), HEADER_SIZE + 4);
    }

    #[test]
    fn round_trip_all_argument_types() {
        let msg = Encoder::begin(7, 3)
            .put_int(-5)
            .put_uint(42)
            .put_fixed(1.5)
            .put_string("hello")
            .put_array(&[1, 2, 3, 4, 5])
            .put_object(9)
            .put_new_id(11)
            .finish()
            .unwrap();

        let (obj, op, body) = decode_message(&msg.bytes).unwrap();
        assert_eq!(obj, 7);
        assert_eq!(op, 3);
        let args = decode_body("iufsaon", body, &[]).unwrap();
        assert_eq!(args.len(), 7);
        assert_eq!(args[0], Arg::Int(-5));
        assert_eq!(args[1], Arg::Uint(42));
        assert_eq!(args[2], Arg::Fixed(1.5));
        assert_eq!(args[3], Arg::Str("hello".into()));
        assert_eq!(args[4], Arg::Array(vec![1, 2, 3, 4, 5]));
        assert_eq!(args[5], Arg::Object(9));
        assert_eq!(args[6], Arg::NewId(11));
    }

    #[test]
    fn array_padding_is_present() {
        let m = Encoder::begin(1, 0)
            .put_array(&[1, 2, 3, 4, 5])
            .finish()
            .unwrap();
        assert_eq!(m.bytes.len(), HEADER_SIZE + 4 + 5 + 3);
    }

    #[test]
    fn fd_argument_takes_no_body_bytes() {
        // This is the detail most likely to be implemented wrongly. "h" must
        // add zero bytes to the body and consume one descriptor.
        let msg = Encoder::begin(1, 0)
            .put_uint(7)
            .put_fd(3)
            .put_uint(8)
            .finish()
            .unwrap();

        assert_eq!(msg.bytes.len(), HEADER_SIZE + 8);
        assert_eq!(msg.fds, vec![3]);

        let (_, _, body) = decode_message(&msg.bytes).unwrap();
        let args = decode_body("uhu", body, &[3]).unwrap();
        assert_eq!(args[0], Arg::Uint(7));
        assert_eq!(args[1], Arg::Fd(3));
        assert_eq!(args[2], Arg::Uint(8));
    }

    #[test]
    fn fd_argument_without_a_descriptor_is_an_error() {
        let msg = Encoder::begin(1, 0).put_fd(3).finish().unwrap();
        let (_, _, body) = decode_message(&msg.bytes).unwrap();
        assert_eq!(decode_body("h", body, &[]), Err(WireError::MissingFd));
    }

    #[test]
    fn rejects_size_smaller_than_header() {
        let mut bytes = vec![0u8; HEADER_SIZE];
        bytes[4..6].copy_from_slice(&4u16.to_ne_bytes());
        assert_eq!(decode_message(&bytes), Err(WireError::BadSize(4)));
    }

    #[test]
    fn rejects_size_not_multiple_of_four() {
        let mut bytes = vec![0u8; 12];
        bytes[4..6].copy_from_slice(&10u16.to_ne_bytes());
        assert_eq!(decode_message(&bytes), Err(WireError::BadSize(10)));
    }

    #[test]
    fn rejects_size_past_the_buffer() {
        let mut bytes = vec![0u8; HEADER_SIZE];
        bytes[4..6].copy_from_slice(&64u16.to_ne_bytes());
        assert!(matches!(
            decode_message(&bytes),
            Err(WireError::Truncated { .. })
        ));
    }

    #[test]
    fn rejects_trailing_bytes() {
        // Two u32 arguments, but the signature only accounts for one, so the
        // second must be reported rather than silently ignored.
        let msg = Encoder::begin(1, 0)
            .put_uint(1)
            .put_uint(2)
            .finish()
            .unwrap();
        let (_, _, body) = decode_message(&msg.bytes).unwrap();
        assert!(matches!(
            decode_body("u", body, &[]),
            Err(WireError::TrailingBytes(4))
        ));
    }

    #[test]
    fn rejects_string_without_nul_terminator() {
        let mut body = Vec::new();
        body.extend_from_slice(&3u32.to_ne_bytes());
        body.extend_from_slice(b"abc");
        body.extend_from_slice(&[0]);
        assert_eq!(decode_body("s", &body, &[]), Err(WireError::BadString));
    }

    #[test]
    fn rejects_string_length_past_end() {
        let mut body = Vec::new();
        body.extend_from_slice(&100u32.to_ne_bytes());
        assert!(matches!(
            decode_body("s", &body, &[]),
            Err(WireError::BadLength { declared: 100, .. })
        ));
    }

    #[test]
    fn oversized_message_is_refused() {
        let mut e = Encoder::begin(1, 0);
        e.buf.resize(u16::MAX as usize + 8, 0);
        assert!(e.finish().is_err());
    }

    #[test]
    fn declared_size_frames_a_stream() {
        let a = Encoder::begin(1, 0).put_uint(1).finish().unwrap();
        let b = Encoder::begin(2, 5).put_string("x").finish().unwrap();
        let mut stream = a.bytes.clone();
        stream.extend_from_slice(&b.bytes);

        let first = declared_size(&stream).unwrap();
        assert_eq!(first, a.bytes.len());
        let (obj, op, _) = decode_message(&stream[..first]).unwrap();
        assert_eq!((obj, op), (1, 0));

        let second = declared_size(&stream[first..]).unwrap();
        let (obj, op, _) = decode_message(&stream[first..first + second]).unwrap();
        assert_eq!((obj, op), (2, 5));
    }
}
