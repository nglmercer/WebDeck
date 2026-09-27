//! Version comparison (extracted from `updater.rs`).

/// Port of the nested `parse_version` + `compare_versions`.
///
/// Returns 1 if `version1` > `version2`, -1 if less, 0 if equal.
/// `-pre`/`-beta` suffixes sort below the plain release.
pub fn compare_versions(version1: &str, version2: &str) -> i32 {
    fn parse_version(version: &str) -> (Vec<u64>, Option<&str>) {
        if let Some(base) = version.strip_suffix("-pre") {
            return (
                base.split('.').filter_map(|p| p.parse().ok()).collect(),
                Some("pre"),
            );
        }
        if let Some(base) = version.strip_suffix("-beta") {
            return (
                base.split('.').filter_map(|p| p.parse().ok()).collect(),
                Some("beta"),
            );
        }
        (
            version.split('.').filter_map(|p| p.parse().ok()).collect(),
            None,
        )
    }

    let (v1, s1) = parse_version(version1);
    let (v2, s2) = parse_version(version2);

    for (a, b) in v1.iter().zip(v2.iter()) {
        if a != b {
            return if a > b { 1 } else { -1 };
        }
    }
    if v1.len() != v2.len() {
        return if v1.len() > v2.len() { 1 } else { -1 };
    }

    let rank = |suffix: Option<&str>| match suffix {
        Some("pre") | Some("beta") => -1,
        _ => 0,
    };
    rank(s1) - rank(s2)
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
        assert_eq!(compare_versions("1.8.7", "1.8"), 1);
        assert_eq!(compare_versions("1.8.7-beta", "1.8.7"), -1);
        assert_eq!(compare_versions("1.8.7-pre", "1.8.7-beta"), 0);
        assert_eq!(compare_versions("1.8.8-beta", "1.8.7"), 1);
    }
}
