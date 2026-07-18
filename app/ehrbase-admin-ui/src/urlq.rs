//! Percent-encoding for query-string VALUES in the console's own internal
//! links (`/queries/aql?aql=…`). A tiny browser-safe helper — deliberately
//! distinct from the server-side wire rule (the `urlencoding` crate, which
//! is `ssr`-gated and covers every CDR-facing path segment). No openEHR
//! spec governs an admin UI's internal links — our own design/extension.

/// Encode a string for use as a URL query-string value: RFC 3986
/// unreserved characters pass; every other byte becomes uppercase `%XX`
/// per UTF-8 byte.
#[must_use]
pub fn encode_query_value(value: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::urlq::encode_query_value;

    #[test]
    fn unreserved_pass_and_everything_else_escapes_per_utf8_byte() {
        assert_eq!(encode_query_value("abc-XYZ_0.9~"), "abc-XYZ_0.9~");
        assert_eq!(
            encode_query_value("a b&c=d%e+f#g?h"),
            "a%20b%26c%3Dd%25e%2Bf%23g%3Fh"
        );
        assert_eq!(encode_query_value("°C"), "%C2%B0C");
        assert_eq!(encode_query_value("é"), "%C3%A9");
    }
}
