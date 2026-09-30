//! Port of `app/buttons/color_picker/get_color_name.py`.
//!
//! `webcolors.hex_to_rgb` is a 3-line parse; ported inline (no dependency).

use serde_json::Value;

fn hex_to_rgb(hex_code: &str) -> Result<(i32, i32, i32), String> {
    let hex = hex_code.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return Err(format!("Invalid hex color: {hex_code}"));
    }
    let channel = |i: usize| {
        i32::from_str_radix(&hex[i..i + 2], 16)
            .map_err(|_| format!("Invalid hex color: {hex_code}"))
    };
    Ok((channel(0)?, channel(2)?, channel(4)?))
}

/// Port of `get_color_name`.
pub fn get_color_name(hex_code: &str, colors: &Value) -> String {
    let empty = Vec::new();
    let colors = colors.as_array().unwrap_or(&empty);

    // Exact match first.
    for color in colors {
        if color.get("hex_code").and_then(|v| v.as_str()) == Some(hex_code) {
            return color
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Can not find color")
                .to_string();
        }
    }

    // Closest color by squared RGB distance.
    let target = match hex_to_rgb(hex_code) {
        Ok(rgb) => rgb,
        Err(_) => return "Can not find color".to_string(),
    };
    let mut closest: Option<&Value> = None;
    let mut min_distance = f64::INFINITY;
    for color in colors {
        let Some(code) = color.get("hex_code").and_then(|v| v.as_str()) else {
            continue;
        };
        let Ok(rgb) = hex_to_rgb(code) else {
            continue;
        };
        let distance = ((target.0 - rgb.0).pow(2)
            + (target.1 - rgb.1).pow(2)
            + (target.2 - rgb.2).pow(2)) as f64;
        if distance < min_distance {
            min_distance = distance;
            closest = Some(color);
        }
    }

    closest
        .and_then(|c| c.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Can not find color")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exact_and_closest() {
        let colors = json!([
            {"hex_code": "#ff0000", "name": "Red"},
            {"hex_code": "#0000ff", "name": "Blue"}
        ]);
        assert_eq!(get_color_name("#ff0000", &colors), "Red");
        assert_eq!(get_color_name("#fe0001", &colors), "Red");
        assert_eq!(get_color_name("not-a-color", &colors), "Can not find color");
    }
}
