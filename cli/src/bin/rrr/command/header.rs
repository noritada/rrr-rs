use crate::common::read_from_source;
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use rrr::json_escape_str;
use std::collections::HashMap;
use std::fmt;

pub(crate) fn cli() -> Command<'static> {
    Command::new("header")
        .about("Display the header of the specified file")
        .arg(
            Arg::new("N")
                .long("bytes")
                .short('b')
                .help("Read only the first N bytes from the S3 bucket")
                .default_value("4096"),
        )
        .arg(
            Arg::new("PATH_OR_URI")
                .help("Path or S3 URI of the file")
                .required(true),
        )
}

pub(crate) async fn exec(args: &ArgMatches) -> Result<()> {
    let fname = args.value_of("PATH_OR_URI").unwrap();
    let n_bytes: usize = args.value_of("N").unwrap().parse()?;
    let (_, header, _) = read_from_source(fname, false, Some(n_bytes)).await?;

    println!("{}", HeaderDisplay(&header));

    Ok(())
}

struct HeaderDisplay<'a>(&'a HashMap<Vec<u8>, Vec<u8>>);

impl<'a> fmt::Display for HeaderDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{")?;
        let Self(inner) = self;
        let mut pair = inner.iter().peekable();
        while let Some((key, val)) = pair.next() {
            let key = String::from_utf8_lossy(key);
            let key = json_escape_str(&key);
            let val = String::from_utf8_lossy(val);
            let val = json_escape_str(&val);
            write!(f, "\"{}\":\"{}\"", key, val)?;
            if pair.peek().is_some() {
                write!(f, ",")?;
            }
        }
        write!(f, "}}")
    }
}
