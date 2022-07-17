use anyhow::anyhow;
use console::Style;
use rrr::{SchemaParseError, SchemaParseErrorKind};

pub(crate) fn create_error_report(err: rrr::Error) -> anyhow::Error {
    match err {
        rrr::Error::Schema(e, bytes) => {
            anyhow!(
                "failed to parse the schema\n\n{}",
                SchemaParseErrorReport(&e, &bytes)
            )
        }
        e => anyhow!("{}", e),
    }
}

pub(crate) struct SchemaParseErrorReport<'e, 'i>(&'e SchemaParseError, &'i [u8]);

impl<'e, 'i> SchemaParseErrorReport<'e, 'i> {
    fn short_reason(&self) -> &'static str {
        let Self(SchemaParseError { kind, .. }, _) = self;
        match kind {
            SchemaParseErrorKind::UnexpectedEof => "unexpected end of the schema statement reached",
            SchemaParseErrorKind::UnexpectedToken => "unexpected token found",
            SchemaParseErrorKind::UnknownBuiltinType => "unknown built type found",
            SchemaParseErrorKind::UnknownToken => "unknown token found",
        }
    }
}

impl<'e, 'i> std::fmt::Display for SchemaParseErrorReport<'e, 'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Self(inner, schema) = self;

        let (lstart, lend) = match inner.kind {
            SchemaParseErrorKind::UnexpectedEof => (inner.location.0, inner.location.0 + 1),
            _ => (inner.location.0, inner.location.1),
        };
        const MARGIN: usize = 16;
        let sstart = std::cmp::max(lstart, MARGIN) - MARGIN;
        let send = std::cmp::min(lend + MARGIN, schema.len());

        let partial_schema: String = schema[sstart..send].iter().map(|b| *b as char).collect();
        let indicator_padding = " ".repeat(lstart - sstart);
        let indicator = "^".repeat(lend - lstart);
        let yellow_bold = Style::new().yellow().bold();
        let bold = Style::new().bold();

        write!(
            f,
            "{}{} {}

    {}
    {}{}
",
            yellow_bold.apply_to("reason"),
            bold.apply_to(":"),
            bold.apply_to(self.short_reason()),
            partial_schema,
            indicator_padding,
            yellow_bold.apply_to(indicator),
        )
    }
}

#[cfg(test)]
mod tests {
    use rrr::Location;

    use super::*;

    macro_rules! test_error_report {
        ($(($name:ident, $input:expr, $kind:ident, $start:expr, $end:expr, $expected:expr),)*) => ($(
            #[test]
            fn $name() {
                let schema_line = $input.as_bytes();
                let error = SchemaParseError {
                    kind: SchemaParseErrorKind::$kind,
                    location: Location($start, $end),
                };
                let report = SchemaParseErrorReport(&error, &schema_line);
                let actual= report.to_string();
                let actual = console::strip_ansi_codes(&actual);
                let expected= $expected;

                assert_eq!(actual, expected);
            }
        )*);
    }

    test_error_report! {
        (report_empty, "", UnexpectedEof, 0, 0,
         "reason: unexpected end of the schema statement reached

    
    ^
"),
        (report_unknown_token, "fld1:%$", UnknownToken, 5, 6,
         "reason: unknown token found

    fld1:%$
         ^
"),
        (report_unexpected_token_at_top_level, "fld1:INT8]", UnexpectedToken, 9, 10,
         "reason: unexpected token found

    fld1:INT8]
             ^
"),
        (report_unknown_builtin_type, "fld1:INT64", UnknownBuiltinType, 5, 10,
         "reason: unknown built type found

    fld1:INT64
         ^^^^^
"),
    }
}
