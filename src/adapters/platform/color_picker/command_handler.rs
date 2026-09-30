//! Port of `app/buttons/color_picker/command_handler.py`.
//!
//! Selection/copy formatting logic is ported 1:1 (including the upstream
//! `TODO: rewrite` and `FIXME` quirks). Screen capture, clipboard writes, and
//! toasts delegate to their ported modules.

use std::collections::HashMap;

use crate::adapters::platform::color_picker::{
    get_arg::getarg, get_color_name::get_color_name, get_mouse_pixel_color::get_mouse_pixel_color,
    notification::toast,
};
use crate::app::utils::{logger::log, translate::translate};

/// Port of `handle_command`.
pub fn handle_command(message: &str) {
    // TODO (upstream): rewrite
    let color = match get_mouse_pixel_color() {
        Ok(color) => color,
        Err(e) => {
            log().error(&format!("colorpicker: cannot capture pixel color: {e}"));
            return;
        }
    };

    let target_language = getarg(message, "lang");
    let selectedtypes = getarg(message, "type");
    let typestocopy = getarg(message, "copy");
    let copy_type = getarg(message, "copy_type");
    let display_type = getarg(message, "display_type");
    let remove_hex_sharp = getarg(message, "remove_hex_sharp").map(|v| {
        let mut chars = v.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    });

    log().debug(&format!(
        "------------------------------------------\n{color:?}\n------------------------------------------\n{target_language:?}\n{selectedtypes:?}\n{typestocopy:?}\n{copy_type:?}\n{display_type:?}\n{remove_hex_sharp:?}\n------------------------------------------"
    ));

    let colorsjson: serde_json::Value = std::fs::read_to_string("webdeck/colors.json")
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or(serde_json::Value::Array(Vec::new()));

    let color_name_original = get_color_name(&color.hex, &colorsjson);
    let mut color_name = color_name_original.clone();
    if target_language.is_some() || target_language.as_deref() == Some("en") {
        if let Some(lang) = target_language.as_deref() {
            color_name = translate(&color_name, lang);
        }
    }

    let mut color_names: HashMap<&str, String> = HashMap::new();
    color_names.insert("NAME", color_name.clone());
    color_names.insert("TEXT", color_name.clone());
    color_names.insert("NAME-ORIGINAL", color_name_original.clone());
    color_names.insert("TEXT-ORIGINAL", color_name_original.clone());
    color_names.insert("HEX", color.hex.clone());
    color_names.insert("RGB", color.rgb.clone());
    color_names.insert("HSL", color.hsl.clone());

    let mut color_names_final: HashMap<String, String> = HashMap::new();
    if let Some(selectedtypes) = selectedtypes.as_deref() {
        for selected in selectedtypes.split(';') {
            for (found, value) in &color_names {
                if found.contains(&selected.to_uppercase()) {
                    if selected.to_uppercase().contains("HEX")
                        && remove_hex_sharp.as_deref() == Some("True")
                    {
                        color_names_final.insert(selected.to_uppercase(), value.replace('#', ""));
                    } else {
                        color_names_final.insert(selected.to_uppercase(), value.clone());
                    }
                }
            }
        }
    } else {
        for (found, value) in &color_names {
            if !found.contains("TEXT") && !found.contains("ORIGINAL") {
                color_names_final.insert(found.to_string(), value.clone());
            }
        }
    }

    if let Some(typestocopy) = typestocopy.as_deref() {
        // FIXME (upstream): WHAT THE FUCK IS THIS
        // copy:text;hex;rgb;hsl copy_type:raw|list
        let mut typestocopy_final: HashMap<String, String> = HashMap::new();
        for selected in typestocopy.split(';') {
            for (found, value) in &color_names {
                if found.contains(&selected.to_uppercase()) {
                    if selected.to_uppercase().contains("HEX")
                        && remove_hex_sharp.as_deref() == Some("True")
                    {
                        typestocopy_final.insert(selected.to_uppercase(), value.replace('#', ""));
                    } else {
                        typestocopy_final.insert(selected.to_uppercase(), value.clone());
                    }
                }
            }
        }
        let parts: Vec<&str> = typestocopy.split(';').collect();
        let text = if copy_type.as_deref().unwrap_or("").to_lowercase() == "list" {
            if parts.len() == 1 {
                // Python: str(dict)[:-2][2:].replace("'", "") — single value.
                typestocopy_final
                    .values()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("")
            } else {
                typestocopy_final
                    .values()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(",\n")
            }
        } else if parts.len() == 1 {
            typestocopy_final
                .values()
                .next()
                .cloned()
                .unwrap_or_default()
        } else {
            typestocopy_final
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        };
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text.clone())) {
            Ok(()) => log().debug(&format!("colorpicker copied: {text}")),
            Err(e) => log().error(&format!("colorpicker clipboard copy failed: {e}")),
        }
    }

    toast(
        display_type.as_deref(),
        typestocopy.as_deref(),
        &color_names_final,
    );
}
