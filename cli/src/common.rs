use anyhow::{anyhow, Result};
use rrr::{DataReader, Schema};
use std::collections::HashMap;
use std::io::{BufRead, Seek};

#[cfg(unix)]
use {pager::Pager, which::which};

pub(crate) async fn read_from_source(
    source: &str,
    with_body: bool,
    n_bytes: Option<&usize>,
) -> Result<(Schema, HashMap<Vec<u8>, Vec<u8>>, Vec<u8>)> {
    if source[0..5] == "s3://"[..] {
        read_from_s3(source, with_body, n_bytes).await
    } else {
        read_from_file(source)
    }
}

async fn read_from_s3(
    url: &str,
    with_body: bool,
    n_bytes: Option<&usize>,
) -> Result<(Schema, HashMap<Vec<u8>, Vec<u8>>, Vec<u8>)> {
    let url = url::Url::parse(url)?;

    let bucket_name = if let Some(url::Host::Domain(s)) = url.host() {
        Ok(s)
    } else {
        Err(anyhow!("bucket name is none"))
    }?;
    let object_key = &url.path()[1..];
    let bytes = download_s3_object(bucket_name, object_key, n_bytes).await?;
    dbg!(bytes.len());

    let f = std::io::Cursor::new(&bytes[..]);
    read_from_reader(f, with_body)
}

async fn download_s3_object(
    bucket_name: &str,
    key: &str,
    n_bytes: Option<&usize>,
) -> Result<bytes::Bytes> {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);

    let req = client.get_object().bucket(bucket_name).key(key);
    let req = if let Some(size) = n_bytes {
        let range = format!("bytes=0-{}", size - 1);
        req.range(range)
    } else {
        req
    };
    let resp = req
        .send()
        .await
        .map_err(crate::diagnostics::create_s3_download_error_report)?;

    let data = resp.body.collect().await?;
    Ok(data.into_bytes())
}

fn read_from_file(fname: &str) -> Result<(Schema, HashMap<Vec<u8>, Vec<u8>>, Vec<u8>)> {
    let input_path = std::path::PathBuf::from(fname);
    let f = std::fs::File::open(input_path)?;
    let f = std::io::BufReader::new(f);
    read_from_reader(f, true)
}

fn read_from_reader<R>(
    reader: R,
    with_body: bool,
) -> Result<(Schema, HashMap<Vec<u8>, Vec<u8>>, Vec<u8>)>
where
    R: BufRead + Seek,
{
    let mut f = DataReader::new(reader);
    f.read(with_body)
        .map_err(crate::diagnostics::create_error_report)
}

#[cfg(unix)]
pub fn start_pager() {
    if which("less").is_ok() {
        Pager::with_pager("less -R").setup();
    } else {
        Pager::new().setup();
    }
}

#[cfg(not(unix))]
pub fn start_pager() {}
