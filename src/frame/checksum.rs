pub fn checksum(data: &[u8]) -> u8 {
    data.iter().fold(0, |acc, &x| acc ^ x)
}
