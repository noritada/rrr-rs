use std::{collections::BTreeMap, fmt};

use anyhow::Result;
use clap::{arg, ArgMatches, Command};
use rrr::{json_escape_str, DataReaderOptions};

use crate::common::read_from_source;

pub(crate) fn cli() -> Command {
    Command::new("header")
        .about("Display the header of the specified file")
        .arg(
            arg!(N: -b --bytes <N> "Read only the first N bytes from the S3 bucket")
                .default_value("4096")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(arg!(<PATH_OR_URI> "Path or S3 URI of the file").required(true))
}

pub(crate) async fn exec(args: &ArgMatches) -> Result<()> {
    let fname = args.get_one::<String>("PATH_OR_URI").unwrap();
    let n_bytes = args.get_one::<usize>("N").unwrap();
    let options = DataReaderOptions::ALLOW_TRAILING_COMMA
        | DataReaderOptions::ALLOW_EMPTY_FIELD_NAME
        | DataReaderOptions::ALLOW_STR_INSTEAD_OF_NSTR;
    let (_, header, _) = read_from_source(fname, Some(n_bytes), options).await?;

    println!("{}", HeaderDisplay(&header));

    Ok(())
}

struct HeaderDisplay<'a>(&'a BTreeMap<Vec<u8>, Vec<u8>>);

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
            write!(f, "\"{key}\":\"{val}\"")?;
            if pair.peek().is_some() {
                write!(f, ",")?;
            }
        }
        write!(f, "}}")
    }
}
