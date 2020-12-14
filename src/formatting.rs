/// Returns the number of bytes scaled down and with the correct prefix and with one digit after
/// the decimal point. For example 1130 would return "1.1 KiB"
pub fn format_bytes_written(bytes: u64) -> String {
    // All the binary unit prefixes needed (u64::MAX == 16 EiB)
    const PREFIXES: &[&str] = &["bytes", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    let mut bytes = bytes as f64;
    let mut prefix_i = 0;
    while bytes >= 1024.0 && prefix_i < PREFIXES.len() - 1 {
        bytes /= 1024.0;
        prefix_i += 1;
    }
    if prefix_i == 0 {
        format!("{:.0} {}", bytes, PREFIXES[prefix_i])
    } else {
        format!("{:.1} {}", bytes, PREFIXES[prefix_i])
    }
}

#[test]
fn test_format_bytes_written() {
    assert_eq!(format_bytes_written(0), "0 bytes");
    assert_eq!(format_bytes_written(1), "1 bytes");
    assert_eq!(format_bytes_written(2), "2 bytes");
    assert_eq!(format_bytes_written(1023), "1023 bytes");
    assert_eq!(format_bytes_written(1024), "1.0 KiB");
    assert_eq!(format_bytes_written(1025), "1.0 KiB");

    assert_eq!(format_bytes_written(1075), "1.0 KiB");
    assert_eq!(format_bytes_written(1076), "1.1 KiB");

    assert_eq!(format_bytes_written(1024 * 1024), "1.0 MiB");

    assert_eq!(format_bytes_written(u64::MAX), "16.0 EiB");
}
