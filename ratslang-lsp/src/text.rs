use tower_lsp_server::ls_types::{Position, Range};

pub fn position(source: &str, byte: usize) -> Position {
    let mut byte = byte.min(source.len());
    while !source.is_char_boundary(byte) {
        byte -= 1;
    }
    let prefix = &source[..byte];
    Position::new(
        prefix.bytes().filter(|b| *b == b'\n').count() as u32,
        prefix
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .trim_end_matches('\r')
            .encode_utf16()
            .count() as u32,
    )
}
pub fn offset(source: &str, at: Position) -> Option<usize> {
    let mut start = 0;
    for _ in 0..at.line {
        start += source.get(start..)?.find('\n')? + 1;
    }
    let line = source
        .get(start..)?
        .split('\n')
        .next()?
        .trim_end_matches('\r');
    let mut units = 0;
    for (byte, ch) in line.char_indices() {
        if units == at.character {
            return Some(start + byte);
        }
        units += ch.len_utf16() as u32;
        if units > at.character {
            return None;
        }
    }
    (units == at.character).then_some(start + line.len())
}
pub fn range(source: &str, bytes: std::ops::Range<usize>) -> Range {
    Range::new(position(source, bytes.start), position(source, bytes.end))
}
pub fn word(source: &str, byte: usize) -> std::ops::Range<usize> {
    fn ident(c: u8) -> bool {
        c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b':')
    }
    let mut start = byte.min(source.len());
    let mut end = start;
    while start > 0 && ident(source.as_bytes()[start - 1]) {
        start -= 1;
    }
    while end < source.len() && ident(source.as_bytes()[end]) {
        end += 1;
    }
    start..end
}
