mod boot;
mod boot_ports;
mod filesystem;
pub mod image;
pub use boot::BootImage;
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect()
}
