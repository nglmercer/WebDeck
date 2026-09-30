//! Version comparison (extracted from `updater.rs`).

/// Port of the nested `parse_version` + `compare_versions`.
///
/// Returns 1 if `version1` > `version2`, -1 if less, 0 if equal.
/// `-pre`/`-beta` suffixes sort below the plain release.
pub fn compare_versions(version1: &str, version2: &str) -> i32 {
    // Release candidates are validated before this boundary. Invalid local
    // version metadata cannot make a candidate appear newer.
    let (Ok(first), Ok(second)) = (
        semver::Version::parse(version1),
        semver::Version::parse(version2),
    ) else {
        return 0;
    };
    match first.cmp(&second) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ordering() {
        assert_eq!(compare_versions("1.8.7", "1.8.7"), 0);
        assert_eq!(compare_versions("1.8.8", "1.8.7"), 1);
        assert_eq!(compare_versions("1.8.6", "1.8.7"), -1);
        assert_eq!(compare_versions("2.0.0", "1.99.99"), 1);
        assert_eq!(compare_versions("1.8.7", "1.8"), 0);
        assert_eq!(compare_versions("1.8.7-beta", "1.8.7"), -1);
        assert_eq!(compare_versions("1.8.7-pre", "1.8.7-beta"), 1);
        assert_eq!(compare_versions("1.8.8-beta", "1.8.7"), 1);
        assert_eq!(compare_versions("2.0.0-alpha.10", "2.0.0-alpha.2"), 1);
        assert_eq!(compare_versions("2.0.0", "2.0.0-alpha.1"), 1);
        assert_eq!(compare_versions("invalid", "2.0.0-alpha.1"), 0);
    }
}
