//! Static-asset helpers (extracted from `server.rs`).
//!
//! Stylesheet SVG collection and portrait rotation for backgrounds.

use crate::app::utils::logger::log;

/// Stylesheet sources scanned by [`get_svgs`]: `style.css` is a manifest —
/// the rules (and their `url(….svg)` references) live in the `@import`ed
/// modules, which resolve relative to `static/css/` in import order.
fn stylesheet_sources() -> Vec<String> {
    let mut sources = Vec::new();
    let Ok(manifest) = std::fs::read_to_string("static/css/style.css") else {
        return sources;
    };
    let mut rest = manifest.as_str();
    let mut found_import = false;
    while let Some(start) = rest.find("@import") {
        rest = &rest[start + "@import".len()..];
        let path = rest
            .trim_start()
            .strip_prefix("url(")
            .and_then(|u| u.split(')').next())
            .or_else(|| {
                rest.trim_start()
                    .split([';', '\n'])
                    .next()
                    .map(|s| s.trim().trim_end_matches(';'))
            })
            .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\''))
            .unwrap_or("");
        // Only local `.css` targets count (the manifest's own header
        // comment mentions `@import` without one).
        if path.ends_with(".css") && !path.starts_with("http") {
            found_import = true;
            if let Ok(content) = std::fs::read_to_string(format!("static/css/{path}")) {
                sources.push(content);
            }
        }
    }
    // No `@import`s (legacy monolith or custom entry): scan it directly.
    if !found_import {
        sources.push(manifest);
    }
    sources
}

/// Port of `get_svgs` — collects `url(….svg)` references from the active
/// stylesheet, following `style.css` `@import`s in cascade order.
pub fn get_svgs() -> Vec<String> {
    let mut svgs = Vec::new();
    for content in stylesheet_sources() {
        let mut rest = content.as_str();
        while let Some(start) = rest.find("url(") {
            rest = &rest[start + 4..];
            let Some(end) = rest.find(')') else {
                break;
            };
            let reference = rest[..end].trim().trim_matches(|c| c == '"' || c == '\'');
            if reference.ends_with(".svg") {
                svgs.push(reference.to_string());
            }
            rest = &rest[end + 1..];
        }
    }
    svgs
}

/// Port of the `img.rotate(-90, expand=True)` blocks: saves a `-90` portrait
/// copy next to the original unless it already exists.
pub(crate) fn save_rotated_copy(original: &str) {
    let path = std::path::Path::new(original);
    let (Some(stem), ext) = (
        path.file_stem().and_then(|s| s.to_str()),
        path.extension().and_then(|s| s.to_str()).unwrap_or(""),
    ) else {
        return;
    };
    let parent = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let rotated_path = if parent.is_empty() {
        format!("{stem}-90.{ext}")
    } else {
        format!("{parent}/{stem}-90.{ext}")
    };
    if std::path::Path::new(&rotated_path).exists() {
        return;
    }
    match image::open(original) {
        Ok(img) => {
            // PIL rotate(-90) = 90° clockwise = rotate270.
            if let Err(e) = img.rotate270().save(&rotated_path) {
                log().exception(
                    &e,
                    Some(&format!("Failed to rotate image {original}")),
                    true,
                    true,
                    true,
                );
            }
        }
        Err(e) => {
            log().exception(
                &e,
                Some(&format!("Failed to rotate image {original}")),
                true,
                true,
                true,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svgs_follow_stylesheet_imports_in_order() {
        // `style.css` is a manifest: the references live in the `@import`ed
        // modules and must surface in cascade order, exactly as the old
        // monolithic file produced them.
        // NOTE: `checked.svg` is gone on purpose — the swap checkbox now
        // renders a text check (contrast fix), so no stylesheet references
        // it and the inventory must not list it.
        assert_eq!(
            get_svgs(),
            vec![
                "/static/img//eye.svg",
                "/static/img//eye-slash.svg",
                "/static/img//chevron_down.svg",
                "/static/img//plus-circle2.svg",
                "/static/img//plus-circle.svg",
                "/static/img//caret-up.svg",
                "/static/img//caret-up.svg",
                "/static/img//caret-up.svg",
                "/static/img//caret-up.svg",
                "/static/img//square.svg",
                "/static/img//square-checked.svg",
            ]
        );
    }

    #[test]
    fn rotated_copy_swaps_dimensions() {
        let dir = std::env::temp_dir().join(format!("webdeck-rot-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("bg.png");
        let img: image::RgbImage =
            image::ImageBuffer::from_fn(4, 2, |x, y| image::Rgb([x as u8, y as u8, 0]));
        img.save(&src).unwrap();
        save_rotated_copy(&src.to_string_lossy());
        let rotated = image::open(dir.join("bg-90.png")).unwrap();
        assert_eq!((rotated.width(), rotated.height()), (2, 4));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
