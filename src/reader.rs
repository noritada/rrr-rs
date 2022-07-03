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
            let compress_type = map.get("compress_type".as_bytes());
            self.read_body(&compress_type)?
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

    fn read_body(&mut self, compress_type: &Option<&Vec<u8>>) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        let buf = match compress_type.map(|s| s.as_slice()) {
            None => {
                self.inner.read_to_end(&mut buf)?;
                buf
            }
            Some(b"gzip") => {
                let mut reader = GzDecoder::new(&mut self.inner);
                reader.read_to_end(&mut buf)?;
                buf
            }
            Some(b"bzip2") => {
                let mut reader = BzDecoder::new(&mut self.inner);
                reader.read_to_end(&mut buf)?;
                buf
            }
            _ => return Err(Error::General),
        };
        Ok(buf)
    }
}
