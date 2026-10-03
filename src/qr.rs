//! Shared QR-viewer helpers (no window code here).
//!
//! Pure logic used by both sides of the split QR popup:
//! - the tray parent ([`crate::app::tray`]) generates the PNG and spawns the
//!   viewer,
//! - the `webdeck-qr` child binary composes the frame and shows it with
//!   `minifb`.
//!
//! Palette is grayscale-only on purpose: `minifb` takes `0xRRGGBB` `u32`
//! pixels, and black/white/gray render identically whatever the channel
//! order of a backend is.

use std::path::{Path, PathBuf};

/// QR bitmap edge, in px (parity with the old webview popup).
pub const QR_DISPLAY_SIZE: u32 = 290;
/// Default viewer window size (parity with the old webview popup).
pub const DEFAULT_WIN_W: u32 = 310;
/// Default viewer window size (parity with the old webview popup).
pub const DEFAULT_WIN_H: u32 = 360;

const WHITE: u32 = 0xffffff;
const BLACK: u32 = 0x000000;

/// Shared QR generation: EC-L PNG bytes for `url`, approximately 290px,
/// black-on-white.
pub fn generate_qr_code_png(url: &str) -> Option<Vec<u8>> {
    let image = render_qr_luma(url, QR_DISPLAY_SIZE)?;
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;
    Some(png)
}

/// Render `text` as a grayscale QR image, at most `max`px on each edge.
pub fn render_qr_luma(text: &str, max: u32) -> Option<image::GrayImage> {
    let code = qrcode::QrCode::with_error_correction_level(text, qrcode::EcLevel::L).ok()?;
    Some(
        code.render::<image::Luma<u8>>()
            .max_dimensions(max, max)
            .build(),
    )
}

/// Parse a `WxH` size such as `310x360`.
pub fn parse_size(raw: &str) -> Option<(u32, u32)> {
    let (w, h) = raw.trim().split_once(['x', 'X'])?;
    let w: u32 = w.trim().parse().ok()?;
    let h: u32 = h.trim().parse().ok()?;
    if !(64..=2048).contains(&w) || !(64..=2048).contains(&h) {
        return None;
    }
    Some((w, h))
}

/// Exit-button rectangle `(x, y, w, h)` for a `w`x`h` window.
pub fn button_rect(w: u32, h: u32) -> (u32, u32, u32, u32) {
    if w < 40 || h < 60 {
        return (0, 0, 0, 0);
    }
    (10, h - 40, w - 20, 30)
}

/// True when point (`x`, `y`) falls inside `rect` (`(x, y, w, h)`).
pub fn hit_test(x: f32, y: f32, rect: (u32, u32, u32, u32)) -> bool {
    let (rx, ry, rw, rh) = rect;
    rw > 0
        && rh > 0
        && x >= rx as f32
        && y >= ry as f32
        && x < (rx + rw) as f32
        && y < (ry + rh) as f32
}

/// 5x7 glyphs, top row first, bit 4 = leftmost pixel. Only the letters
/// needed for the `EXIT` button plus space.
fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        _ => [0; 7],
    }
}

/// Pixel width of `text` at integer `scale` (5px glyph + 1px gap).
pub fn text_width(text: &str, scale: u32) -> u32 {
    let n = text.chars().count() as u32;
    if n == 0 {
        return 0;
    }
    n * 6 * scale - scale
}

/// Draw `text` (5x7 font) with top-left at (`x0`, `y0`), clipped to the buffer.
#[allow(clippy::too_many_arguments)]
pub fn draw_text(
    buf: &mut [u32],
    w: u32,
    h: u32,
    text: &str,
    x0: u32,
    y0: u32,
    color: u32,
    scale: u32,
) {
    let scale = scale.max(1);
    for (i, ch) in text.chars().enumerate() {
        let glyph = glyph(ch.to_ascii_uppercase());
        let base_x = x0 + i as u32 * 6 * scale;
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let x = base_x + col * scale + dx;
                        let y = y0 + row as u32 * scale + dy;
                        if x < w && y < h {
                            buf[(y * w + x) as usize] = color;
                        }
                    }
                }
            }
        }
    }
}

/// Compose a `w`x`h` `0xRRGGBB` frame: white background, `qr` centered at
/// the top, black `EXIT` button at the bottom.
pub fn compose_frame(qr: &image::GrayImage, w: u32, h: u32) -> Vec<u32> {
    let mut buf = vec![WHITE; (w * h) as usize];

    let q = QR_DISPLAY_SIZE
        .min(w.saturating_sub(20))
        .min(h.saturating_sub(80));
    if q > 0 {
        let (sw, sh) = (qr.width(), qr.height());
        let x0 = (w - q) / 2;
        let y0 = 8;
        for dy in 0..q {
            for dx in 0..q {
                let luma = qr.get_pixel(dx * sw / q, dy * sh / q).0[0];
                let x = x0 + dx;
                let y = y0 + dy;
                if x < w && y < h {
                    buf[(y * w + x) as usize] = if luma < 128 { BLACK } else { WHITE };
                }
            }
        }
    }

    let (bx, by, bw, bh) = button_rect(w, h);
    if bw > 0 && bh > 0 {
        for y in by..by + bh {
            for x in bx..bx + bw {
                buf[(y * w + x) as usize] = BLACK;
            }
        }
        let scale = if bw >= 120 && bh >= 24 { 2 } else { 1 };
        let tw = text_width("EXIT", scale);
        let th = 7 * scale;
        draw_text(
            &mut buf,
            w,
            h,
            "EXIT",
            bx + (bw - tw) / 2,
            by + (bh - th) / 2,
            WHITE,
            scale,
        );
    }

    buf
}

/// Absolute path of the PNG handoff file (`temp/qr.png`, gitignored).
pub fn qr_png_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("temp")
        .join("qr.png")
}

/// `stem` binary sitting next to the `exe` binary (release layout ships
/// `WebDeck` + `webdeck-qr` side by side).
pub fn sibling_binary(exe: &Path, stem: &str) -> PathBuf {
    let name = format!("{stem}{}", std::env::consts::EXE_SUFFIX);
    exe.parent()
        .map(|p| p.join(&name))
        .unwrap_or_else(|| PathBuf::from(name))
}

/// Locate the `webdeck-qr` viewer: sibling of the current executable when
/// present, otherwise a bare `webdeck-qr` PATH lookup.
pub fn resolve_viewer_binary() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let sibling = sibling_binary(&exe, "webdeck-qr");
        if sibling.is_file() {
            return sibling;
        }
    }
    PathBuf::from("webdeck-qr")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_parses_w_by_h() {
        assert_eq!(parse_size("310x360"), Some((310, 360)));
        assert_eq!(parse_size(" 800X600 "), Some((800, 600)));
    }

    #[test]
    fn size_rejects_garbage_and_absurd() {
        for raw in [
            "", "310", "x360", "310x", "axb", "10x10", "9999x10", "0x100", "-1x100",
        ] {
            assert_eq!(parse_size(raw), None, "input: {raw:?}");
        }
    }

    #[test]
    fn button_geometry_matches_default_window() {
        assert_eq!(button_rect(310, 360), (10, 320, 290, 30));
        assert!(hit_test(150.0, 335.0, button_rect(310, 360)));
        assert!(!hit_test(150.0, 319.0, button_rect(310, 360)));
        assert!(!hit_test(5.0, 335.0, button_rect(310, 360)));
        assert!(!hit_test(305.0, 335.0, button_rect(310, 360)));
    }

    #[test]
    fn tiny_window_has_no_button() {
        assert_eq!(button_rect(30, 30), (0, 0, 0, 0));
        assert!(!hit_test(5.0, 5.0, button_rect(30, 30)));
    }

    #[test]
    fn glyph_e_sets_18_pixels_at_scale_1() {
        let mut buf = vec![WHITE; 8 * 8];
        draw_text(&mut buf, 8, 8, "E", 0, 0, BLACK, 1);
        assert_eq!(buf.iter().filter(|&&p| p == BLACK).count(), 18);
    }

    #[test]
    fn unknown_glyph_is_blank_but_advances() {
        assert_eq!(text_width("E?T", 1), 3 * 6 - 1);
        let mut buf = vec![WHITE; 20 * 8];
        draw_text(&mut buf, 20, 8, "?", 0, 0, BLACK, 1);
        assert!(buf.iter().all(|&p| p == WHITE));
    }

    #[test]
    fn frame_has_qr_button_and_white_corners() {
        let qr = render_qr_luma("http://127.0.0.1:8080/", 290).expect("qr renders");
        let frame = compose_frame(&qr, 310, 360);
        assert_eq!(frame.len(), 310 * 360);
        assert_eq!(frame[0], WHITE);
        assert_eq!(frame[309], WHITE);
        // QR area holds both black modules and white background.
        let qr_area = || frame.iter().skip(8 * 310).take(290 * 310);
        assert!(qr_area().any(|&p| p == BLACK));
        assert!(qr_area().any(|&p| p == WHITE));
        // Button area is black with a white EXIT label.
        let (bx, by, bw, bh) = button_rect(310, 360);
        let mut black = 0;
        let mut white = 0;
        for y in by..by + bh {
            for x in bx..bx + bw {
                match frame[(y * 310 + x) as usize] {
                    BLACK => black += 1,
                    WHITE => white += 1,
                    other => panic!("unexpected pixel {other:#x}"),
                }
            }
        }
        assert!(black > 1000 && white > 50, "black={black} white={white}");
    }

    #[test]
    fn qr_png_bytes_have_png_magic() {
        let png = generate_qr_code_png("http://127.0.0.1:8080/").expect("qr png");
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }

    #[test]
    fn sibling_binary_keeps_exe_dir() {
        let exe = Path::new("/opt/webdeck/WebDeck");
        let expected = format!("/opt/webdeck/webdeck-qr{}", std::env::consts::EXE_SUFFIX);
        assert_eq!(sibling_binary(exe, "webdeck-qr"), PathBuf::from(expected));
    }
}
