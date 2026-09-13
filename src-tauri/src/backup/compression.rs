use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::prelude::*;

/// Magic bytes for gzip (RFC 1952)
const GZIP_MAGIC_BYTE_1: u8 = 0x1f;
const GZIP_MAGIC_BYTE_2: u8 = 0x8b;

/// Compresses a raw database byte slice using Gzip.
pub fn compress_db(bytes: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes)?;
    encoder.finish()
}

/// Decompresses a database byte slice if it is Gzip compressed (detected via magic bytes).
/// If the data is not gzipped (e.g. legacy raw SQLite), it returns the original bytes untouched.
pub fn decompress_db_if_needed(bytes: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    if is_gzipped(bytes) {
        let mut decoder = GzDecoder::new(bytes);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    } else {
        Ok(bytes.to_vec())
    }
}

/// Checks whether the given byte slice begins with Gzip magic bytes.
pub fn is_gzipped(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == GZIP_MAGIC_BYTE_1 && bytes[1] == GZIP_MAGIC_BYTE_2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_and_decompress() {
        let original = b"SQLite format 3\0 - Mock Database Content with Notes and Tasks!";
        let compressed = compress_db(original).expect("Compression failed");

        assert!(is_gzipped(&compressed));
        assert!(compressed.len() > 0);

        let decompressed = decompress_db_if_needed(&compressed).expect("Decompression failed");
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_legacy_uncompressed_backward_compatibility() {
        let legacy_data = b"SQLite format 3\0 - Legacy uncompressed raw file from earlier version";
        assert!(!is_gzipped(legacy_data));

        let result = decompress_db_if_needed(legacy_data).expect("Should handle legacy safely");
        assert_eq!(result, legacy_data);
    }

    #[test]
    fn test_empty_or_short_slices() {
        assert!(!is_gzipped(&[]));
        assert!(!is_gzipped(&[0x1f]));

        let empty_result = decompress_db_if_needed(&[]).unwrap();
        assert_eq!(empty_result, Vec::<u8>::new());
    }
}
