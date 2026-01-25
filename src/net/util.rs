pub fn net_util_data_hexdump(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02X}", byte)).collect()
}
pub fn net_util_data_ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| {
            if !byte.is_ascii_graphic() && !byte.is_ascii_whitespace() {
                '.'
            } else {
                *byte as char
            }
        })
        .collect()
}
