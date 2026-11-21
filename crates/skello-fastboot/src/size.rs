/// Parses fastboot max-download-size values: `0x10000000`, `536870912`,
/// `64M`, `512K`, `1G`. Returns bytes, or None if the value is unreadable.
pub fn parse_download_size(value: &str) -> Option<u64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16).ok();
    }
    let (number, multiplier) = match value.chars().last()? {
        'K' | 'k' => (&value[..value.len() - 1], 1024u64),
        'M' | 'm' => (&value[..value.len() - 1], 1024 * 1024),
        'G' | 'g' => (&value[..value.len() - 1], 1024 * 1024 * 1024),
        _ => (value, 1),
    };
    number.trim().parse::<u64>().ok().map(|n| n * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_decimal_and_suffixed_sizes() {
        assert_eq!(parse_download_size("0x10000000"), Some(0x10000000));
        assert_eq!(parse_download_size("536870912"), Some(536870912));
        assert_eq!(parse_download_size("64M"), Some(67108864));
        assert_eq!(parse_download_size("512K"), Some(524288));
        assert_eq!(parse_download_size("1G"), Some(1073741824));
    }

    #[test]
    fn tolerates_spaces_and_case() {
        assert_eq!(parse_download_size(" 0x1fe00000 "), Some(0x1fe00000));
        assert_eq!(parse_download_size("64m"), Some(67108864));
    }

    #[test]
    fn rejects_junk_and_empty_values() {
        assert_eq!(parse_download_size(""), None);
        assert_eq!(parse_download_size("unknown"), None);
        assert_eq!(parse_download_size("0x"), None);
    }
}
