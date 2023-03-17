use std::{
    collections::HashMap,
    io::{BufRead, Read, Seek, SeekFrom},
};

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
pub use options::DataReaderOptions;

use crate::{ast::Schema, Error};

mod options;

pub struct DataReader<R> {
    inner: R,
    options: DataReaderOptions,
}

impl<R> DataReader<R> {
    const START_MAGIC: &'static [u8] = "WN\n".as_bytes();
    const START_MAGIC_LEN: usize = Self::START_MAGIC.len();
    const SEP_MAGIC: &'static [u8] = [0x04, 0x1a].as_slice();
    const SEP_MAGIC_LEN: usize = Self::SEP_MAGIC.len();

    pub fn new(inner: R, options: DataReaderOptions) -> Self {
        Self { inner, options }
    }
}

impl<R> DataReader<R>
where
    R: BufRead + Seek,
{
    pub fn read(&mut self) -> Result<(Schema, HashMap<Vec<u8>, Vec<u8>>, Vec<u8>), Error> {
        self.inner.rewind()?;
        self.find_magic()?;
        let map = self.read_header_fields()?;

        let schema = map.get_required_field("format")?;
        let schema: Schema = schema.as_slice().try_into()?;

        let body = if self
            .options
            .contains(DataReaderOptions::ENABLE_READING_BODY)
        {
            let body_size = map.get_required_field("data_size")?;
            let body_size = String::from_utf8_lossy(body_size)
                .parse::<usize>()
                .map_err(|_| Error::from_str(r#""data_size" value is not an integer"#))?;
            let compress_type = map.get_field("compress_type");
            self.read_body(body_size, &compress_type)?
        } else {
            Vec::new()
        };

        Ok((schema, map.inner(), body))
    }

    fn find_magic(&mut self) -> Result<usize, Error> {
        let mut buf = Vec::new();
        loop {
            let len = self.inner.read_until(b'\n', &mut buf)?;
            if len == 0 {
                return Err(Error::from_str(r#"magic "WN\n" not found"#));
            }
            let buf_len = buf.len();
            if buf_len >= Self::START_MAGIC_LEN
                && buf[buf_len - Self::START_MAGIC_LEN..] == *Self::START_MAGIC
            {
                return Ok(buf_len);
            }
        }
    }

    fn read_header_fields(&mut self) -> Result<FieldMap, Error> {
        let mut sep_buf = vec![0; Self::SEP_MAGIC_LEN];
        let mut map = HashMap::new();

        loop {
            self.inner
                .read_exact(&mut sep_buf)
                .map_err(|_| Error::from_str("unexpected EOF in reading the header"))?;
            if sep_buf == Self::SEP_MAGIC {
                break;
            }
            self.inner
                .seek(SeekFrom::Current(-(Self::SEP_MAGIC_LEN as i64)))?;

            let mut buf = Vec::new();
            loop {
                let len = self.inner.read_until(b'\n', &mut buf)?;
                if len == 0 {
                    return Err(Error::from_str("unexpected EOF in reading the header"));
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
                return Err(Error::from_str(
                    "invalid line without an equal character found in the header",
                ));
            }
        }

        Ok(FieldMap(map))
    }

    fn read_body(
        &mut self,
        body_size: usize,
        compress_type: &Option<&Vec<u8>>,
    ) -> Result<Vec<u8>, Error> {
        // We want to report how many bytes are actually read when the buffer is not
        // filled, although `read_exact` does not report it.
        // So, we use `read_to_end` here, assuming that the data is correctly ended.
        let mut buf = Vec::with_capacity(body_size);
        self.inner
            .read_to_end(&mut buf)
            .map_err(|e| Error::from_string(format!("reading body failed: {e}")))?;
        if !self
            .options
            .contains(DataReaderOptions::IGNORE_DATA_SIZE_FIELD)
        {
            let len = buf.len();
            if len < body_size {
                return Err(Error::from_string(format!(
                    "unexpected EOF in reading body: {len} bytes read; {body_size} bytes expected"
                )));
            }
            buf.truncate(body_size);
        };

        let buf = match compress_type.map(|s| s.as_slice()) {
            None => buf,
            Some(b"gzip") => {
                let mut reader = GzDecoder::new(&buf[..]);
                let mut decoded = Vec::new();
                reader.read_to_end(&mut decoded).map_err(|e| {
                    Error::from_string(format!("reading gzip-compressed body failed: {e}"))
                })?;
                decoded
            }
            Some(b"bzip2") => {
                let mut reader = BzDecoder::new(&buf[..]);
                let mut decoded = Vec::new();
                reader.read_to_end(&mut decoded).map_err(|e| {
                    Error::from_string(format!("reading bzip2-compressed body failed: {e}"))
                })?;
                decoded
            }
            Some(s) => {
                let s = String::from_utf8_lossy(s);
                return Err(Error::from_string(format!(
                    "unknown \"compress_type\" field value: {s}"
                )));
            }
        };
        Ok(buf)
    }
}

struct FieldMap(HashMap<Vec<u8>, Vec<u8>>);

impl FieldMap {
    fn inner(self) -> HashMap<Vec<u8>, Vec<u8>> {
        let Self(inner) = self;
        inner
    }
    fn get_field(&self, name: &str) -> Option<&Vec<u8>> {
        let Self(inner) = self;
        inner.get(name.as_bytes())
    }

    fn get_required_field(&self, name: &str) -> Result<&Vec<u8>, Error> {
        self.get_field(name)
            .ok_or_else(|| Error::from_string(format!("\"{name}\" field not found")))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    macro_rules! test_read_errors {
        ($((
            $name:ident,
            $header:expr,
            $expected:expr
        ),)*) => ($(
            #[test]
            fn $name() {
                let options = DataReaderOptions::ENABLE_READING_BODY;
                let mut reader = DataReader::new(Cursor::new($header), options);
                let actual = reader.read().map(|(_, _, _)| ());
                assert_eq!(actual, $expected);
            }
        )*);
    }

    test_read_errors! {
        (read_error_for_empty_data, b"", Err(Error::from_str(r#"magic "WN\n" not found"#))),
        (read_error_for_too_short_data, b"WN", Err(Error::from_str(r#"magic "WN\n" not found"#))),
        (
            read_error_for_data_without_magic,
            b"abcde",
            Err(Error::from_str(r#"magic "WN\n" not found"#))
        ),
        (
            read_error_for_data_with_eof_before_newline,
            b"WN
data_size=0
format=field:UINT8",
            Err(Error::from_str("unexpected EOF in reading the header"))
        ),
        (
            read_error_for_data_with_eof_before_newline_with_escaped_newlines,
            b"WN
data_size=0
f\\\normat\\\n=\\\nfield:\\\nUINT8\\\n",
            Err(Error::from_str("unexpected EOF in reading the header"))
        ),
        (
            read_error_for_data_with_eof_before_separator_magic,
            b"WN
data_size=0
format=field:UINT8
",
            Err(Error::from_str("unexpected EOF in reading the header"))
        ),
        (
            read_error_for_data_with_eof_before_separator_magic_with_escaped_newlines,
            b"WN
data_size=0
f\\\normat\\\n=\\\nfield:\\\nUINT8\\\n
",
            Err(Error::from_str("unexpected EOF in reading the header"))
        ),
        (
            no_read_error_for_minimal_data,
            b"WN
data_size=0
format=field:UINT8
\x04\x1a",
            Ok(())
        ),
        (
            no_read_error_for_data_with_escaped_newlines,
            b"WN
data_size=0
f\\\normat\\\n=\\\nfield:\\\nUINT8\\\n
\x04\x1a",
            Ok(())
        ),
        (
            read_error_for_data_with_invalid_line,
            b"WN
data_size=0
format=field1:UINT8
field2:UINT8
\x04\x1a",
            Err(Error::from_str("invalid line without an equal character found in the header"))
        ),
        (
            read_errors_for_data_without_schema,
            b"WN
data_size=0
\x04\x1a",
            Err(Error::from_str(r#""format" field not found"#))
        ),
        (
            read_errors_for_data_without_body_size,
            b"WN
format=field:UINT8
\x04\x1a",
            Err(Error::from_str(r#""data_size" field not found"#))
        ),
        (
            read_errors_for_data_with_wrong_body_size,
            b"WN
data_size=0byte
format=field:UINT8
\x04\x1a",
            Err(Error::from_str(r#""data_size" value is not an integer"#))
        ),
    }

    fn uncompressed_body_data() -> Vec<u8> {
        b"\x00\x01\x02\x03".to_vec()
    }

    fn gzip_compressed_body_data() -> Vec<u8> {
        b"\
\x1f\x8b\x08\x08\x37\xd5\x67\x63\x02\xff\x66\x69\x6c\x65\x00\x63\
\x60\x64\x62\x06\x00\x13\x86\xb9\x8b\x04\x00\x00\x00"
            .to_vec()
    }

    fn bzip2_compressed_body_data() -> Vec<u8> {
        b"\
\x42\x5a\x68\x39\x31\x41\x59\x26\x53\x59\x94\x92\x36\xd5\x00\x00\
\x00\x40\x00\x78\x00\x20\x00\x21\x9a\x68\x33\x4d\x13\x3c\x5d\xc9\
\x14\xe1\x42\x42\x52\x48\xdb\x54"
            .to_vec()
    }

    macro_rules! test_data_size_handling_for_uncompressed_body {
        ($((
            $name:ident,
            $body:expr,
            $num_extra_bytes:expr,
            $data_size_field_ignored:expr,
            $compress_type_field:expr,
            $expected:expr
        ),)*) => ($(
            #[test]
            fn $name() {
                let body = $body;
                let body_size = body.len() as isize + $num_extra_bytes;
                let compress_type_field = $compress_type_field;
                let header = format!(
                    "WN
data_size={body_size}
format=field:{{10}}UINT8
{compress_type_field}\x04\x1a"
                );
                let bytes = [header.as_bytes(), &body].concat();

                let options = DataReaderOptions::ENABLE_READING_BODY;
                let options = if $data_size_field_ignored {
                    options.union(DataReaderOptions::IGNORE_DATA_SIZE_FIELD)
                } else {
                    options
                };
                let mut reader = DataReader::new(Cursor::new(&bytes), options);
                let actual_body = reader.read().map(|(_, _, body_returned)| body_returned);
                assert_eq!(actual_body, $expected);
            }
        )*);
    }

    test_data_size_handling_for_uncompressed_body! {
        (
            data_size_handling_for_uncompressed_body_with_no_extra_bytes,
            uncompressed_body_data(),
            0,
            false,
            "",
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_uncompressed_body_with_negative_extra_bytes,
            uncompressed_body_data(),
            -1,
            false,
            "",
            Ok(b"\x00\x01\x02".to_vec())
        ),
        (
            data_size_handling_for_uncompressed_body_with_negative_extra_bytes_ignoring_field_value,
            uncompressed_body_data(),
            -1,
            true,
            "",
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_uncompressed_body_with_positive_extra_bytes,
            uncompressed_body_data(),
            1,
            false,
            "",
            Err(crate::Error::from_str(
                "unexpected EOF in reading body: 4 bytes read; 5 bytes expected"
            ))
        ),
        (
            data_size_handling_for_uncompressed_body_with_positive_extra_bytes_ignoring_field_value,
            uncompressed_body_data(),
            1,
            true,
            "",
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_gzip_compressed_body_with_no_extra_bytes,
            gzip_compressed_body_data(),
            0,
            false,
            "compress_type=gzip\n",
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_gzip_compressed_body_with_negative_extra_bytes,
            gzip_compressed_body_data(),
            -1,
            false,
            "compress_type=gzip\n",
            Err(crate::Error::from_str(
                "reading gzip-compressed body failed: unexpected end of file"
            ))
        ),
        (
            data_size_handling_for_gzip_compressed_body_with_negative_extra_bytes_ignoring_field_value,
            gzip_compressed_body_data(),
            -1,
            true,
            "compress_type=gzip\n",
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_gzip_compressed_body_with_positive_extra_bytes,
            gzip_compressed_body_data(),
            1,
            false,
            "compress_type=gzip\n",
            Err(crate::Error::from_str(
                "unexpected EOF in reading body: 29 bytes read; 30 bytes expected"
            ))
        ),
        (
            data_size_handling_for_gzip_compressed_body_with_positive_extra_bytes_ignoring_field_value,
            gzip_compressed_body_data(),
            1,
            true,
            "compress_type=gzip\n",
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_bzip2_compressed_body_with_no_extra_bytes,
            bzip2_compressed_body_data(),
            0,
            false,
            "compress_type=bzip2\n",
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_bzip2_compressed_body_with_negative_extra_bytes,
            bzip2_compressed_body_data(),
            -1,
            false,
            "compress_type=bzip2\n",
            Err(crate::Error::from_str(
                "reading bzip2-compressed body failed: decompression not finished but EOF reached"
            ))
        ),
        (
            data_size_handling_for_bzip2_compressed_body_with_negative_extra_bytes_ignoring_field_value,
            bzip2_compressed_body_data(),
            -1,
            true,
            "compress_type=bzip2\n",
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_bzip2_compressed_body_with_positive_extra_bytes,
            bzip2_compressed_body_data(),
            1,
            false,
            "compress_type=bzip2\n",
            Err(crate::Error::from_str(
                "unexpected EOF in reading body: 40 bytes read; 41 bytes expected"
            ))
        ),
        (
            data_size_handling_for_bzip2_compressed_body_with_positive_extra_bytes_ignoring_field_value,
            bzip2_compressed_body_data(),
            1,
            true,
            "compress_type=bzip2\n",
            Ok(b"\x00\x01\x02\x03".to_vec())
        ),
        (
            data_size_handling_for_gzip_decoding_of_bzip2_compressed_data,
            bzip2_compressed_body_data(),
            0,
            false,
            "compress_type=gzip\n",
            Err(crate::Error::from_str("reading gzip-compressed body failed: invalid gzip header"))
        ),
        (
            data_size_handling_for_bzip2_decoding_of_gzip_compressed_data,
            gzip_compressed_body_data(),
            0,
            false,
            "compress_type=bzip2\n",
            Err(crate::Error::from_str(
                "reading bzip2-compressed body failed: bzip2: bz2 header missing"
            ))
        ),
        (
            data_size_handling_for_unknown_compress_type,
            uncompressed_body_data(),
            0,
            false,
            "compress_type=xz\n",
            Err(crate::Error::from_str("unknown \"compress_type\" field value: xz"))
        ),
    }
}
