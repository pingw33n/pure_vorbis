use std::cmp;
use std::io::{Error, ErrorKind, Read, Result};

use util::Bits;

/// A `Read`-like trait that works on a bit level as specified by [Bitpacking Convention].
/// [Bitpacking Convention]: https://www.xiph.org/vorbis/doc/Vorbis_I_spec.html#x1-360002
pub trait BitRead: Read {
    /// Atempts reading at most `len_bits` and returns the bits read as `u32` value and the number of
    /// bits read as `usize`.
    fn try_read_u32_bits(&mut self, len_bits: usize) -> Result<(u32, usize)>;

    /// Reads exactly `len_bits` and returns the bits read as `u32` value or `ErrorKind::UnexpectedEof`
    /// if it wasn't possible to read enough bits.
    fn read_u32_bits(&mut self, len_bits: usize) -> Result<u32> {
        let (r, r_len) = try!(self.try_read_u32_bits(len_bits));
        if r_len == len_bits {
            Ok(r)
        } else {
            Err(Error::new(ErrorKind::UnexpectedEof, "Couldn't read enough bits"))
        }
    }

    /// Pushes back the `bits` into internal buffer. The buffered bits will be read again by successive
    /// [try_read_u32_bits()](#tymethod.try_read_u32_bits) calls.
    /// # Panics
    /// Panics if the `len_bits` and the existing buffered bits form a value wider than 64 bits.
    /// Effectively this means it's not possible to unread more than 32 bits.
    fn unread_u32_bits(&mut self, bits: u32, len_bits: usize);

    fn read_u8_bits(&mut self, len_bits: usize) -> Result<u8> {
        assert!(len_bits <= 8);
        self.read_u32_bits(len_bits).map(|v| v as u8)
    }

    fn read_u8(&mut self) -> Result<u8> {
        self.read_u8_bits(8)
    }

    fn read_u16_bits(&mut self, len_bits: usize) -> Result<u16> {
        assert!(len_bits <= 16);
        self.read_u32_bits(len_bits).map(|v| v as u16)
    }

    fn read_u16(&mut self) -> Result<u16> {
        self.read_u16_bits(16)
    }

    fn read_i32_bits(&mut self, len_bits: usize) -> Result<i32> {
        assert!(len_bits >= 2);
        let u = try!(self.read_u32_bits(len_bits - 1));
        let sign = try!(self.read_bool());
        if sign {
            Ok(-(u as i32))
        } else {
            Ok(u as i32)
        }
    }

    fn read_u32(&mut self) -> Result<u32> {
        self.read_u32_bits(32)
    }

    fn read_i32(&mut self) -> Result<i32> {
        self.read_i32_bits(32)
    }

    // Reads one bit and treats it as `false` if it's 0 or `true` if it's 1.
    fn read_bool(&mut self) -> Result<bool> {
        self.read_u8_bits(1).map(|v| v & 1 == 1)
    }

    /// Reads `f32` value as defined by [float32_unpack](https://www.xiph.org/vorbis/doc/Vorbis_I_spec.html#x1-1200009.2.2).
    fn read_f32(&mut self) -> Result<f32> {
        self.read_u32().map(|v| f32_unpack(v))
    }
}

pub struct BitReader<R> {
    inner: R,
    bit_buf: u64,
    bit_buf_left: usize,
}

impl<R: Read> BitReader<R> {
    pub fn new(reader: R) -> Self {
        BitReader {
            inner: reader,
            bit_buf: 0,
            bit_buf_left: 0,
        }
    }

    fn fill_bit_buf(&mut self) -> Result<()> {
        assert_eq!(self.bit_buf_left, 0);
        // Intentionally reading only 32 bits saving another 32 bits for the unread buffer.
        let mut buf = [0; 4];
        let read = try!(self.inner.read(&mut buf));
        self.bit_buf_left = read * 8;

        if read == 0 {
            return Ok(());
        }

        let mut bit_buf = buf[0] as u64;
        if read == 1 {
            self.bit_buf = bit_buf;
            return Ok(());
        }

        bit_buf |= (buf[1] as u64) << 8;
        if read == 2 {
            self.bit_buf = bit_buf;
            return Ok(());
        }

        bit_buf |= (buf[2] as u64) << 16;
        if read == 3 {
            self.bit_buf = bit_buf;
            return Ok(());
        }

        bit_buf |= (buf[3] as u64) << 24;
        self.bit_buf = bit_buf;

        Ok(())
    }

    fn read_bit_buf(&mut self, target: &mut u32, offset: usize, len: usize) -> usize {
        assert!(offset + len <= 32);
        if len == 0 || self.bit_buf_left == 0 {
            return 0;
        }
        let can_read = cmp::min(self.bit_buf_left, len);
        let bits = (self.bit_buf as u32).ls_bits(can_read);
        *target = if offset == 0 {
            bits
        } else {
            target.ls_bits(offset) | (bits << offset)
        };
        if can_read == self.bit_buf_left {
            self.bit_buf = 0;
            self.bit_buf_left = 0;
        } else {
            self.bit_buf >>= can_read;
            self.bit_buf_left -= can_read;
        }
        can_read
    }
}

impl<R: Read> BitRead for BitReader<R> {
    fn try_read_u32_bits(&mut self, len_bits: usize) -> Result<(u32, usize)> {
        if len_bits == 0 {
            return Ok((0, 0));
        }
        assert!(len_bits <= 32);
        if self.bit_buf_left == 0 {
            try!(self.fill_bit_buf());
        }
        let mut r = 0;
        let mut read_bits = self.read_bit_buf(&mut r, 0, len_bits);
        if read_bits != 0 && read_bits < len_bits && self.bit_buf_left == 0 {
            try!(self.fill_bit_buf());
            read_bits += self.read_bit_buf(&mut r, read_bits, len_bits - read_bits);
        }
        Ok((r, read_bits))
    }

    fn unread_u32_bits(&mut self, bits: u32, len_bits: usize) {
        if len_bits == 0 {
            return;
        }
        assert!(self.bit_buf_left + len_bits <= 64);
        self.bit_buf = (self.bit_buf << len_bits) | bits.ls_bits(len_bits) as u64;
        self.bit_buf_left += len_bits;
    }
}

impl<R: Read> Read for BitReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }

        for i in 0..buf.len() {
            buf[i] = try!(self.read_u8());
        }

        Ok(buf.len())
    }
}

fn f32_unpack(val: u32) -> f32 {
    let mut mantissa = (val & 0x1F_FFFF) as f32;
    let sign = val & 0x8000_0000;
    if sign != 0 {
        mantissa = -mantissa;
    }
    let exponent = ((val & 0x7FE0_0000) >> 21) as f32;
    mantissa * 2_f32.powf(exponent - 788_f32)
}

#[cfg(test)]
mod tests {
    use std::io::{ErrorKind, Cursor, Read};

    use super::{BitRead, BitReader};

    #[test]
    fn try_read_u32_bits() {
        let mut r = BitReader::new(Cursor::new([0b001_00110]));
        assert_eq!(r.try_read_u32_bits(5).unwrap(), (0b00110, 5));
        assert_eq!(r.try_read_u32_bits(32).unwrap(), (0b001, 3));
    }

    #[test]
    fn read_u32_bits_var() {
        let mut r = BitReader::new(Cursor::new([0b0_0100110, 0b0111_0011, 0b0110_1001]));
        assert_eq!(r.read_u32_bits(7).unwrap(), 0b0100110);
        assert_eq!(r.read_u32_bits(5).unwrap(), 0b00110);
        assert_eq!(r.read_u32_bits(4).unwrap(), 0b0111);
        assert_eq!(r.read_u32_bits(4).unwrap(), 0b1001);
        assert_eq!(r.read_u32_bits(5).unwrap_err().kind(), ErrorKind::UnexpectedEof);
    }

    #[test]
    fn read_u32_bits_10_1() {
        let mut r = BitReader::new(Cursor::new([0b00100110, 0b011100_11, 0b0000_1001, 0, 0]));
        assert_eq!(r.read_u32_bits(10).unwrap(), 0b1100100110);
        assert_eq!(r.read_u32_bits(10).unwrap(), 0b1001011100);
    }

    #[test]
    fn read_u32_bits_10_2() {
        let mut r = BitReader::new(Cursor::new([0b01011101, 0b010111_00, 0b0100_0000, 0b10010111]));
        assert_eq!(r.read_u32_bits(10).unwrap(), 0b0001011101);
        assert_eq!(r.read_u32_bits(10).unwrap(), 0b0000010111);
        assert_eq!(r.read_u32_bits(10).unwrap(), 0b0101110100);
    }

    #[test]
    fn read_u32_bits_second_read() {
        let mut r = BitReader::new(Cursor::new([0b01011101, 0b01011100, 0b01000000, 0b10010111,
                                                0b00100110]));
        assert_eq!(r.read_u32_bits(25).unwrap(), 0b1_01000000_01011100_01011101);
        assert_eq!(r.read_u32_bits(9).unwrap(), 0b10_1001011);
        assert_eq!(r.read_u32_bits(6).unwrap(), 0b001001);
        assert_eq!(r.read_u32_bits(1).unwrap_err().kind(), ErrorKind::UnexpectedEof);
    }

    #[test]
    fn read_i32_bits() {
        let mut r = BitReader::new(Cursor::new([0b01_011_101, 0b11011100]));
        assert_eq!(r.read_i32_bits(3).unwrap(), -0b001);
        assert_eq!(r.read_i32_bits(3).unwrap(),  0b011);
        assert_eq!(r.read_i32_bits(9).unwrap(), -0b001110001);
    }

    #[test]
    fn read_u8() {
        let inp = [0b11111100, 0b01001000, 0b11001110, 0b00000110];
        let mut r = BitReader::new(Cursor::new(&inp));
        for e in &inp {
            assert_eq!(r.read_u8().unwrap(), *e)
        }
    }

    #[test]
    fn unread_u32_bits() {
        let inp = [0b01011101, 0b01011100, 0b01000000, 0b10010111,
                   0b00100110];
        let mut r = BitReader::new(Cursor::new(&inp));
        assert_eq!(r.read_u8().unwrap(), 0b01011101);
        r.unread_u32_bits(0b01011101, 8);
        assert_eq!(r.read_u32_bits(25).unwrap(), 0b1_01000000_01011100_01011101);
        r.unread_u32_bits(0b1_01000000_01011100_01011101, 25);

        let mut act = [0_u8; 5];
        r.read_exact(&mut act).unwrap();
        assert_eq!(act, inp);
    }

    #[test]
    fn read() {
        let mut r = BitReader::new(Cursor::new([0b00100110, 0b01110011, 0b011_01001, 0b100_10011,
                                                0b101_10010]));
        let mut buf = [0; 2];

        assert_eq!(r.read(&mut buf).unwrap(), 2);
        assert_eq!(buf, [0b00100110, 0b01110011]);

        assert_eq!(r.read_u32_bits(5).unwrap(), 0b01001);

        assert_eq!(r.read(&mut buf).unwrap(), 2);
        assert_eq!(buf, [0b10011011, 0b10010100]);

        assert_eq!(r.read_u32_bits(3).unwrap(), 0b101);

        assert_eq!(r.read_u32_bits(1).unwrap_err().kind(), ErrorKind::UnexpectedEof);
    }
}