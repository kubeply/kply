//! Configuration primitives for future Kply project and cluster settings.

/// Canonical Kply project configuration filename.
pub const CANONICAL_CONFIG_FILENAME: &str = "kply.yaml";

#[cfg(test)]
mod tests {
    use super::CANONICAL_CONFIG_FILENAME;

    #[test]
    fn uses_kply_yaml_as_canonical_config_filename() {
        assert_eq!(CANONICAL_CONFIG_FILENAME, "kply.yaml");
    }
}
