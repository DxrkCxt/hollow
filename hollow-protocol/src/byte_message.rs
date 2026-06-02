// SPDX-License-Identifier: GPL-3.0-only

//! Only the protocol read/write
//! operations actually used (not the ~1000 lines of ByteBuf delegate methods).
//!
//! Writes are infallible (append to a BytesMut). Reads are fallible (returning
//! `DecodeError` where the Java original throws DecoderException).

use bytes::{BufMut, BytesMut};
use crab_nbt::NbtCompound;
use uuid::Uuid;

use crate::registry::Version;
use crate::nbt;

const DEFAULT_MAX_STRING: usize = 32767; // Short.MAX_VALUE

#[derive(Debug, Clone)]
pub struct DecodeError(pub String);

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for DecodeError {}

pub type DecodeResult<T> = Result<T, DecodeError>;

fn err<T>(msg: impl Into<String>) -> DecodeResult<T> {
    Err(DecodeError(msg.into()))
}

pub struct ByteMessage {
    buf: BytesMut,
    reader: usize,
}

impl Default for ByteMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteMessage {
    pub fn new() -> ByteMessage {
        ByteMessage {
            buf: BytesMut::new(),
            reader: 0,
        }
    }

    /// Wrap received bytes for decoding.
    pub fn from_bytes(data: &[u8]) -> ByteMessage {
        ByteMessage {
            buf: BytesMut::from(data),
            reader: 0,
        }
    }

    /// Wrap an already-owned buffer for decoding without copying. Used by the read
    /// loop, where the frame is a `BytesMut` split off the read buffer (zero-copy).
    pub fn from_buf(buf: BytesMut) -> ByteMessage {
        ByteMessage { buf, reader: 0 }
    }

    pub fn readable_bytes(&self) -> usize {
        self.buf.len() - self.reader
    }

    /// Equivalent to toByteArray(): the unread remainder (the whole buffer when encoding).
    pub fn to_byte_array(&self) -> Vec<u8> {
        self.buf[self.reader..].to_vec()
    }

    pub fn as_read_slice(&self) -> &[u8] {
        &self.buf[self.reader..]
    }

    // ---- primitive reads ----

    fn ensure(&self, n: usize) -> DecodeResult<()> {
        if self.readable_bytes() < n {
            err(format!("Buffer underflow: need {n}, have {}", self.readable_bytes()))
        } else {
            Ok(())
        }
    }

    pub fn read_u8(&mut self) -> DecodeResult<u8> {
        self.ensure(1)?;
        let b = self.buf[self.reader];
        self.reader += 1;
        Ok(b)
    }

    pub fn read_bool(&mut self) -> DecodeResult<bool> {
        Ok(self.read_u8()? != 0)
    }

    fn read_n<const N: usize>(&mut self) -> DecodeResult<[u8; N]> {
        self.ensure(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(&self.buf[self.reader..self.reader + N]);
        self.reader += N;
        Ok(out)
    }

    pub fn read_u16(&mut self) -> DecodeResult<u16> {
        Ok(u16::from_be_bytes(self.read_n::<2>()?))
    }
    pub fn read_i32(&mut self) -> DecodeResult<i32> {
        Ok(i32::from_be_bytes(self.read_n::<4>()?))
    }
    pub fn read_i64(&mut self) -> DecodeResult<i64> {
        Ok(i64::from_be_bytes(self.read_n::<8>()?))
    }

    pub fn read_bytes(&mut self, n: usize) -> DecodeResult<Vec<u8>> {
        self.ensure(n)?;
        let v = self.buf[self.reader..self.reader + n].to_vec();
        self.reader += n;
        Ok(v)
    }

    pub fn read_var_int(&mut self) -> DecodeResult<i32> {
        let readable = self.readable_bytes();
        if readable == 0 {
            return err("Empty buffer");
        }
        let k = self.read_u8()? as i32;
        if (k & 0x80) != 0x80 {
            return Ok(k);
        }
        let max_read = readable.min(5);
        let mut i = k & 0x7F;
        let mut j = 1usize;
        while j < max_read {
            let k2 = self.read_u8()? as i32;
            i |= (k2 & 0x7F) << (j * 7);
            if (k2 & 0x80) != 0x80 {
                return Ok(i);
            }
            j += 1;
        }
        err("Bad VarInt")
    }

    pub fn read_string(&mut self) -> DecodeResult<String> {
        self.read_string_max(DEFAULT_MAX_STRING)
    }

    pub fn read_string_max(&mut self, max_len: usize) -> DecodeResult<String> {
        let len = self.read_var_int()?;
        if len < 0 {
            return err("Negative string length");
        }
        let len = len as usize;
        if len > max_len * 3 {
            return err(format!(
                "Cannot receive string longer than {} (got {len} bytes)",
                max_len * 3
            ));
        }
        let bytes = self.read_bytes(len)?;
        let s = String::from_utf8(bytes).map_err(|_| DecodeError("Invalid UTF-8 string".into()))?;
        if s.chars().count() > max_len {
            return err(format!(
                "Cannot receive string longer than {max_len} characters"
            ));
        }
        Ok(s)
    }

    pub fn read_uuid(&mut self) -> DecodeResult<Uuid> {
        let msb = self.read_i64()? as u64;
        let lsb = self.read_i64()? as u64;
        Ok(Uuid::from_u64_pair(msb, lsb))
    }

    /// Equivalent to readArray(limit): a varint-prefixed byte blob limited in size.
    pub fn read_array_limited(&mut self, limit: usize) -> DecodeResult<Vec<u8>> {
        let len = self.read_var_int()?;
        if len < 0 {
            return err("Negative array length");
        }
        let len = len as usize;
        if len > limit {
            return err(format!(
                "Cannot receive byte array longer than {limit} (got {len} bytes)"
            ));
        }
        self.read_bytes(len)
    }

    // ---- primitive writes ----

    pub fn write_u8(&mut self, v: u8) {
        self.buf.put_u8(v);
    }
    pub fn write_i8(&mut self, v: i8) {
        self.buf.put_i8(v);
    }
    pub fn write_bool(&mut self, v: bool) {
        self.buf.put_u8(if v { 1 } else { 0 });
    }
    pub fn write_i16(&mut self, v: i16) {
        self.buf.put_i16(v);
    }
    pub fn write_i32(&mut self, v: i32) {
        self.buf.put_i32(v);
    }
    pub fn write_i64(&mut self, v: i64) {
        self.buf.put_i64(v);
    }
    pub fn write_f32(&mut self, v: f32) {
        self.buf.put_f32(v);
    }
    pub fn write_f64(&mut self, v: f64) {
        self.buf.put_f64(v);
    }
    pub fn write_bytes(&mut self, data: &[u8]) {
        self.buf.put_slice(data);
    }

    pub fn write_var_int(&mut self, value: i32) {
        put_var_int(&mut self.buf, value);
    }

    pub fn write_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.write_var_int(bytes.len() as i32);
        self.buf.put_slice(bytes);
    }

    pub fn write_uuid(&mut self, uuid: Uuid) {
        let (msb, lsb) = uuid.as_u64_pair();
        self.buf.put_u64(msb);
        self.buf.put_u64(lsb);
    }

    /// Fixed-length bitset of `size` bits, little-endian within bytes, padded to
    /// `(size + 8) >> 3` bytes — matching ByteMessage.writeFixedBitSet exactly.
    pub fn write_fixed_bit_set(&mut self, bits: &[bool], size: usize) {
        let num_bytes = (size + 8) >> 3;
        let mut bytes = vec![0u8; num_bytes];
        for (i, &b) in bits.iter().enumerate() {
            if b {
                bytes[i >> 3] |= 1 << (i & 7);
            }
        }
        self.buf.put_slice(&bytes);
    }

    pub fn write_namespaced_key(&mut self, key: &str) {
        self.write_string(key);
    }

    pub fn write_compound_tag(&mut self, compound: &NbtCompound, version: Version) {
        let bytes = nbt::write_compound(compound, version);
        self.buf.put_slice(&bytes);
    }
}

/// Write a VarInt into any `BufMut` — `BytesMut` or `Vec<u8>`. Sharing this lets the
/// frame encoder write the length prefix straight into the outbound `Vec` instead of
/// through an intermediate `ByteMessage` that would copy the payload again.
pub fn put_var_int<B: BufMut>(buf: &mut B, value: i32) {
    let uv = value as u32;
    if (value & (!0i32 << 7)) == 0 {
        buf.put_u8(value as u8);
    } else if (value & (!0i32 << 14)) == 0 {
        let w = ((value & 0x7F | 0x80) << 8) | ((uv >> 7) as i32);
        buf.put_u16(w as u16);
    } else if (value & (!0i32 << 21)) == 0 {
        let w = ((value & 0x7F | 0x80) << 16)
            | ((((uv >> 7) as i32) & 0x7F | 0x80) << 8)
            | ((uv >> 14) as i32);
        buf.put_uint((w as u64) & 0xFF_FFFF, 3);
    } else if (value & (!0i32 << 28)) == 0 {
        let w = ((value & 0x7F | 0x80) << 24)
            | ((((uv >> 7) as i32) & 0x7F | 0x80) << 16)
            | ((((uv >> 14) as i32) & 0x7F | 0x80) << 8)
            | ((uv >> 21) as i32);
        buf.put_i32(w);
    } else {
        let w = ((value & 0x7F | 0x80) << 24)
            | ((((uv >> 7) as i32) & 0x7F | 0x80) << 16)
            | ((((uv >> 14) as i32) & 0x7F | 0x80) << 8)
            | (((uv >> 21) as i32) & 0x7F | 0x80);
        buf.put_i32(w);
        buf.put_u8((uv >> 28) as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_var_int(v: i32) {
        let mut m = ByteMessage::new();
        m.write_var_int(v);
        let bytes = m.to_byte_array();
        let mut r = ByteMessage::from_bytes(&bytes);
        assert_eq!(r.read_var_int().unwrap(), v, "value {v}");
        assert_eq!(r.readable_bytes(), 0);
    }

    #[test]
    fn var_int_roundtrip() {
        for v in [0, 1, 127, 128, 255, 256, 16383, 16384, 2097151, 2097152, i32::MAX, -1, i32::MIN] {
            roundtrip_var_int(v);
        }
    }

    #[test]
    fn var_int_known_encodings() {
        let mut m = ByteMessage::new();
        m.write_var_int(0);
        m.write_var_int(127);
        m.write_var_int(128);
        m.write_var_int(255);
        assert_eq!(m.to_byte_array(), vec![0x00, 0x7F, 0x80, 0x01, 0xFF, 0x01]);
    }

    #[test]
    fn put_var_int_matches_write_var_int() {
        // The frame encoder writes the length prefix via `put_var_int` straight into a
        // Vec; it must be byte-identical to the `ByteMessage::write_var_int` path.
        for v in [
            0, 1, 127, 128, 255, 256, 16383, 16384, 2097151, 2097152, 268435455, 268435456,
            i32::MAX, -1, i32::MIN,
        ] {
            let mut m = ByteMessage::new();
            m.write_var_int(v);
            let via_method = m.to_byte_array();

            let mut via_fn: Vec<u8> = Vec::new();
            put_var_int(&mut via_fn, v);

            assert_eq!(via_fn, via_method, "value {v}");
        }
    }

    #[test]
    fn string_roundtrip() {
        let mut m = ByteMessage::new();
        m.write_string("Hello, мир!");
        let bytes = m.to_byte_array();
        let mut r = ByteMessage::from_bytes(&bytes);
        assert_eq!(r.read_string().unwrap(), "Hello, мир!");
    }

    #[test]
    fn uuid_roundtrip() {
        let u = Uuid::from_u128(0x0123456789abcdef_fedcba9876543210);
        let mut m = ByteMessage::new();
        m.write_uuid(u);
        let mut r = ByteMessage::from_bytes(&m.to_byte_array());
        assert_eq!(r.read_uuid().unwrap(), u);
    }
}
