use std::str;

pub fn strtod_manual(input: &[u8]) -> Option<f64> {
    let input_str = str::from_utf8(input).ok()?;

    // Extract the numeric prefix
    let numeric_part: String = input_str.chars().take_while(|c| c.is_digit(10)).collect();

    if numeric_part.is_empty() {
        None
    } else {
        // TODO: can alrdy parse here to LNum?
        let parsed = numeric_part.parse::<f64>().ok()?;
        Some(parsed)
    }
}
