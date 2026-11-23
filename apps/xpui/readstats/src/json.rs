//! Just enough JSON to read the log's lines and the sync settings.
//!
//! A plain substring search for `"key":` is sound here: a quote inside a
//! title or path is escaped (`\"`), so the pattern can't match inside a string.

/// What follows `key` in `text`.
pub fn after<'a>(text: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    text.windows(key.len())
        .position(|window| window == key)
        .map(|position| &text[position + key.len()..])
}

/// The integer after `key`; 0 when it is missing.
pub fn number(text: &[u8], key: &[u8]) -> i64 {
    let Some(value) = after(text, key) else {
        return 0;
    };
    let negative = value.first() == Some(&b'-');
    let digits = value[usize::from(negative)..]
        .iter()
        .take(12)
        .take_while(|byte| byte.is_ascii_digit());
    let magnitude = digits.fold(0i64, |total, digit| total * 10 + i64::from(digit - b'0'));
    if negative { -magnitude } else { magnitude }
}

/// The string after `key` (`"name":`, spaces allowed), with `\"` and `\\`
/// unescaped, written into `out`; returns its length, or `None` when it is
/// missing, unfinished or longer than `out`.
pub fn string(text: &[u8], key: &[u8], out: &mut [u8]) -> Option<usize> {
    let value = after(text, key)?.trim_ascii_start();
    let [b'"', rest @ ..] = value else {
        return None;
    };
    let (mut len, mut bytes) = (0, rest.iter());
    while let Some(&byte) = bytes.next() {
        let byte = match byte {
            b'"' => return Some(len),
            b'\\' => *bytes.next()?,
            byte => byte,
        };
        *out.get_mut(len)? = byte;
        len += 1;
    }
    None
}
