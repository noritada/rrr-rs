use crate::ast::Schema;
use crate::Error;
use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::io::{BufRead, Read, Seek, SeekFrom};

pub struct DataReader<R> {
    inner: R,
}

impl<R> DataReader<R> {
    const START_MAGIC: &'static [u8] = "WN\n".as_bytes();
    const START_MAGIC_LEN: usize = Self::START_MAGIC.len();
    const SEP_MAGIC: &'static [u8] = [0x04, 0x1a].as_slice();
    const SEP_MAGIC_LEN: usize = Self::SEP_MAGIC.len();

    pub fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R> DataReader<R>
where
    R: BufRead + Seek,
{
    pub fn read(
        &mut self,
        with_body: bool,
    ) -> Result<(Schema, HashMap<Vec<u8>, Vec<u8>>, Vec<u8>), Error> {
        self.inner.seek(SeekFrom::Start(0))?;
        self.find_magic()?;
        let map = self.read_header_fields()?;

        let schema = map.get("format".as_bytes()).ok_or(Error::General)?;
        let schema: Schema = schema.as_slice().try_into()?;

        let body = if with_body {
            let body_size = map.get("data_size".as_bytes()).ok_or(Error::General)?;
            let body_size = String::from_utf8_lossy(body_size)
                .parse::<usize>()
                .or_else(|_| Err(Error::General))?;
            let compress_type = map.get("compress_type".as_bytes());
            self.read_body(body_size, &compress_type)?
        } else {
            Vec::new()
        };

        Ok((schema, map, body))
    }

    fn find_magic(&mut self) -> Result<usize, Error> {
        let mut buf = Vec::new();
        loop {
            let len = self.inner.read_until(b'\n', &mut buf)?;
            if len == 0 {
                return Err(Error::General); // magic not found
            }
            let buf_len = buf.len();
            if buf_len >= Self::START_MAGIC_LEN
                && buf[buf_len - Self::START_MAGIC_LEN..] == *Self::START_MAGIC
            {
                return Ok(buf_len);
            }
        }
    }

    fn read_header_fields(&mut self) -> Result<HashMap<Vec<u8>, Vec<u8>>, Error> {
        let mut sep_buf = vec![0; Self::SEP_MAGIC_LEN];
        let mut map = HashMap::new();

        loop {
            self.inner.read_exact(&mut sep_buf)?;
            if sep_buf == Self::SEP_MAGIC {
                break;
            }
            self.inner
                .seek(SeekFrom::Current(-(Self::SEP_MAGIC_LEN as i64)))?;

            let mut buf = Vec::new();
            loop {
                let len = self.inner.read_until(b'\n', &mut buf)?;
                if len == 0 {
                    return Err(Error::General); // EOF in reading the header
                }
                let buf_len = buf.len();
                if buf_len < 2 || buf[buf_len - 2] != b'\\' {
                    break;
                }
                buf.pop();
                buf.pop();
            }

            buf.pop(); // remove a trailing newline
            if let Some(pos) = buf.iter().position(|&b| b == b'=') {
                let val = buf.split_off(pos + 1);
                buf.pop(); // remove b'='
                map.insert(buf, val);
            } else {
                return Err(Error::General);
            }
        }

        Ok(map)
    }

    fn read_body(
        &mut self,
        body_size: usize,
        compress_type: &Option<&Vec<u8>>,
    ) -> Result<Vec<u8>, Error> {
        let mut buf = vec![0; body_size];
        self.inner.read_exact(&mut buf)?;
        let buf = match compress_type.map(|s| s.as_slice()) {
            None => buf,
            Some(b"gzip") => {
                let mut reader = GzDecoder::new(&buf[..]);
                let mut decoded = Vec::new();
                reader.read_to_end(&mut decoded)?;
                decoded
            }
            Some(b"bzip2") => {
                let mut reader = BzDecoder::new(&buf[..]);
                let mut decoded = Vec::new();
                reader.read_to_end(&mut decoded)?;
                decoded
            }
            _ => return Err(Error::General),
        };
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    macro_rules! test_data_size_handling_for_uncompressed_body {
        ($(($name:ident, $num_extra_bytes:expr, $expected_body:expr),)*) => ($(
            #[test]
            fn $name() {
                let body = b"\x00\x01\x02\x03".to_vec();
                let body_size = body.len() as isize + $num_extra_bytes;
                let header = format!(
                    "WN
data_size={body_size}
format=field:{{10}}UINT8
\x04\x1a"
                );
                let bytes = [header.as_bytes(), &body].concat();

                let mut reader = DataReader::new(Cursor::new(&bytes));
                let actual_body = reader.read(true).map(|(_, _, body_returned)| body_returned);
                assert_eq!(actual_body, $expected_body);
            }
        )*);
    }

    test_data_size_handling_for_uncompressed_body! {
        (
            data_size_handling_for_uncompressed_body_with_no_extra_bytes,
            0,
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_uncompressed_body_with_negative_extra_bytes,
            -1,
            Ok(b"\x00\x01\x02".to_vec())
        ),
        (
            data_size_handling_for_uncompressed_body_with_positive_extra_bytes,
            1,
            Err(crate::Error::General)
        ),
    }

    macro_rules! test_data_size_handling_for_gzip_compressed_body {
        ($(($name:ident, $num_extra_bytes:expr, $expected_body:expr),)*) => ($(
            #[test]
            fn $name() {
                let body = (b"\
\x1f\x8b\x08\x08\x37\xd5\x67\x63\x02\xff\x66\x69\x6c\x65\x00\x63\
\x60\x64\x62\x06\x00\x13\x86\xb9\x8b\x04\x00\x00\x00")
                    .to_vec();
                let body_size = body.len() as isize + $num_extra_bytes;
                let header = format!(
                    "WN
data_size={body_size}
compress_type=gzip
format=field:{{10}}UINT8
\x04\x1a"
                );
                let bytes = [header.as_bytes(), &body].concat();

                let mut reader = DataReader::new(Cursor::new(&bytes));
                let actual_body = reader.read(true).map(|(_, _, body_returned)| body_returned);
                assert_eq!(actual_body, $expected_body);
            }
        )*);
    }

    test_data_size_handling_for_gzip_compressed_body! {
        (
            data_size_handling_for_gzip_compressed_body_with_no_extra_bytes,
            0,
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_gzip_compressed_body_with_negative_extra_bytes,
            -1,
            Err(crate::Error::General)
        ),
        (
            data_size_handling_for_gzip_compressed_body_with_positive_extra_bytes,
            1,
            Err(crate::Error::General)
        ),
    }

    macro_rules! test_data_size_handling_for_bzip2_compressed_body {
        ($(($name:ident, $num_extra_bytes:expr, $expected_body:expr),)*) => ($(
            #[test]
            fn $name() {
                let body = (b"\
\x42\x5a\x68\x39\x31\x41\x59\x26\x53\x59\x94\x92\x36\xd5\x00\x00\
\x00\x40\x00\x78\x00\x20\x00\x21\x9a\x68\x33\x4d\x13\x3c\x5d\xc9\
\x14\xe1\x42\x42\x52\x48\xdb\x54")
                    .to_vec();
                let body_size = body.len() as isize + $num_extra_bytes;
                let header = format!(
                    "WN
data_size={body_size}
compress_type=bzip2
format=field:{{10}}UINT8
\x04\x1a"
                );
                let bytes = [header.as_bytes(), &body].concat();

                let mut reader = DataReader::new(Cursor::new(&bytes));
                let actual_body = reader.read(true).map(|(_, _, body_returned)| body_returned);
                assert_eq!(actual_body, $expected_body);
            }
        )*);
    }

    test_data_size_handling_for_bzip2_compressed_body! {
        (
            data_size_handling_for_bzip2_compressed_body_with_no_extra_bytes,
            0,
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_bzip2_compressed_body_with_negative_extra_bytes,
            -1,
            Err(crate::Error::General)
        ),
        (
            data_size_handling_for_bzip2_compressed_body_with_positive_extra_bytes,
            1,
            Err(crate::Error::General)
        ),
    }
}
