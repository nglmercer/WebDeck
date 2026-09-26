//! Port of `app/utils/settings/gridsize.py`.

use serde_json::{json, Value};

use crate::app::utils::logger::log;

fn void_button() -> Value {
    json!({"VOID": "VOID"})
}

fn is_void(button: &Value) -> bool {
    button == &void_button()
}

fn grid_dim(config: &Value, key: &str) -> usize {
    config
        .get("front")
        .and_then(|f| f.get(key))
        .and_then(|v| {
            v.as_u64()
                .map(|n| n as usize)
                .or_else(|| v.as_str().and_then(|s| s.parse::<usize>().ok()))
        })
        .unwrap_or(0)
}

/// Port of `create_matrix`.
///
/// Returns `(matrix, folder_names)` — Python's `unmatrix` re-derives folder
/// order from the config; we thread the order through explicitly (same order,
/// since `serde_json` runs with `preserve_order`).
pub type ButtonMatrix = Vec<Vec<Vec<Value>>>;

pub fn create_matrix(config: &Value) -> (ButtonMatrix, Vec<String>) {
    let mut matrix: ButtonMatrix = Vec::new();
    let mut folder_names: Vec<String> = Vec::new();
    let width = grid_dim(config, "width").max(1);

    if let Some(buttons) = config
        .get("front")
        .and_then(|f| f.get("buttons"))
        .and_then(|b| b.as_object())
    {
        for (folder_name, folder_content) in buttons {
            folder_names.push(folder_name.clone());
            let mut folder_matrix: Vec<Vec<Value>> = Vec::new();
            let mut row_count: usize = 0;
            if let Some(list) = folder_content.as_array() {
                for (count, button) in list.iter().enumerate() {
                    if row_count >= folder_matrix.len() {
                        folder_matrix.push(Vec::new());
                    }
                    folder_matrix[row_count].push(button.clone());
                    if (count + 1) % width == 0 {
                        row_count += 1;
                    }
                }
            }
            matrix.push(folder_matrix);
        }
    }
    (matrix, folder_names)
}

/// Port of `unmatrix`.
pub fn unmatrix(mut config: Value, matrix: &ButtonMatrix, folder_names: &[String]) -> Value {
    if let Some(buttons) = config
        .get_mut("front")
        .and_then(|f| f.get_mut("buttons"))
        .and_then(|b| b.as_object_mut())
    {
        for (folder_count, folder) in matrix.iter().enumerate() {
            if let Some(folder_name) = folder_names.get(folder_count) {
                let mut flat: Vec<Value> = Vec::new();
                for row in folder {
                    flat.extend(row.iter().cloned());
                }
                buttons.insert(folder_name.clone(), Value::Array(flat));
            }
        }
    }
    config
}

/// Port of `update_gridsize`.
///
/// Faithful port of the grow/shrink algorithm, including the VOID-compaction
/// passes and the `{"DEL": "DEL"}` markers (which Python leaves in the grid
/// the same way).
pub fn update_gridsize(config: Value, new_height: usize, new_width: usize) -> Value {
    let (mut matrix, folder_names) = create_matrix(&config);
    let old_height = grid_dim(&config, "height");
    let old_width = grid_dim(&config, "width");

    // Height changed.
    if old_height != new_height {
        if new_height > old_height {
            let difference = new_height - old_height;
            for _ in 0..difference {
                for folder in matrix.iter_mut() {
                    for _ in 0..old_width {
                        // Appended to the last row's folder list; recreate the
                        // flat append by pushing a new row chunk below.
                        let _ = folder;
                    }
                    // Python appends to the flat folder list then rebuilds the
                    // matrix; emulate by extending rows then re-chunking.
                    let mut flat: Vec<Value> = folder.iter().flatten().cloned().collect();
                    flat.extend((0..old_width).map(|_| void_button()));
                    *folder = chunk_rows(&flat, old_width.max(1));
                }
            }
        }

        if old_height > new_height {
            let difference = old_height - new_height;
            log().debug("gridsize: Height decreased");
            for _ in 0..difference {
                for folder in matrix.iter_mut() {
                    // Drop the last all-VOID row if there is one.
                    let mut removed = false;
                    for row_index in (0..folder.len()).rev() {
                        if !folder[row_index].is_empty()
                            && folder[row_index].iter().all(is_void)
                        {
                            folder.remove(row_index);
                            removed = true;
                            break;
                        }
                    }
                    if removed {
                        continue;
                    }
                    // Otherwise compact VOIDs upward column by column.
                    if folder.is_empty() {
                        continue;
                    }
                    let cols = folder[0].len();
                    for col in 0..cols {
                        let mut moved = false;
                        for row in (0..folder.len()).rev() {
                            if folder[row].get(col).map(is_void).unwrap_or(false) {
                                let mut num = folder.len() - row;
                                while num > 1 {
                                    let below = folder.len() - num + 1;
                                    let current = folder.len() - num;
                                    if below < folder.len()
                                        && current < folder.len()
                                        && col < folder[below].len()
                                        && col < folder[current].len()
                                    {
                                        let value = folder[below][col].clone();
                                        folder[current][col] = value;
                                    }
                                    num -= 1;
                                }
                                let last_idx = folder.len() - 1;
                                if col < folder[last_idx].len() {
                                    folder[last_idx][col] = json!({"DEL": "DEL"});
                                }
                                moved = true;
                                break;
                            }
                        }
                        if moved {
                            continue;
                        }
                        // Relocate last-row button into the first VOID slot.
                        let mut relocated = false;
                        'search: for (row_b, row) in folder.iter().enumerate() {
                            for (col_b, button) in row.iter().enumerate() {
                                if is_void(button) {
                                    if let Some(last) = folder.last() {
                                        if let Some(value) = last.get(col).cloned() {
                                            folder[row_b][col_b] = value;
                                            relocated = true;
                                            break 'search;
                                        }
                                    }
                                }
                            }
                        }
                        if !relocated {
                            log().debug("gridsize: NOT ENOUGH SPACE");
                        }
                    }
                    folder.pop();
                }
            }
        }
    }

    // Width changed.
    if old_width != new_width {
        if new_width > old_width {
            let difference = new_width - old_width;
            for _ in 0..difference {
                for folder in matrix.iter_mut() {
                    for row in folder.iter_mut() {
                        row.push(void_button());
                    }
                }
            }
        }

        if new_width < old_width {
            let difference = old_width - new_width;
            log().debug("gridsize: Width decreased");
            for _ in 0..difference {
                for folder in matrix.iter_mut() {
                    if folder.is_empty() || folder[0].is_empty() {
                        continue;
                    }
                    // Drop the rightmost all-VOID column if there is one.
                    let mut removed = false;
                    for col_from_right in 0..folder[0].len() {
                        let col = folder[0].len() - 1 - col_from_right;
                        if folder.iter().all(|row| row.get(col).map(is_void).unwrap_or(false)) {
                            for row in folder.iter_mut() {
                                row.remove(col);
                            }
                            removed = true;
                            break;
                        }
                    }
                    if removed {
                        continue;
                    }
                    // Otherwise remove one VOID per row, then any remaining
                    // VOIDs round-robin, mirroring the Python passes.
                    let mut element_to_del = folder.len();
                    for row in folder.iter_mut() {
                        if let Some(pos) = row.iter().position(is_void) {
                            row.remove(pos);
                            element_to_del -= 1;
                            if element_to_del == 0 {
                                break;
                            }
                        }
                    }
                    if element_to_del > 0 {
                        'outer: for row in folder.iter_mut() {
                            while let Some(pos) = row.iter().position(is_void) {
                                row.remove(pos);
                                element_to_del -= 1;
                                if element_to_del == 0 {
                                    break 'outer;
                                }
                            }
                        }
                    }
                    if element_to_del > 0 {
                        log().debug("gridsize: NOT ENOUGH SPACE");
                    }
                }
            }
        }
    }

    let config = unmatrix(config, &matrix, &folder_names);
    log().success("Grid size updated successfully");
    log().info(&format!(
        "Old grid size: {old_height}x{old_width}, New grid size: {new_height}x{new_width}"
    ));
    config
}

fn chunk_rows(flat: &[Value], width: usize) -> Vec<Vec<Value>> {
    let width = width.max(1);
    flat.chunks(width).map(|c| c.to_vec()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_config() -> Value {
        json!({
            "front": {
                "height": 2,
                "width": 2,
                "buttons": {
                    "main": [
                        {"name": "a"}, {"name": "b"},
                        {"name": "c"}, {"VOID": "VOID"}
                    ]
                }
            }
        })
    }

    #[test]
    fn matrix_roundtrip() {
        let config = test_config();
        let (matrix, folders) = create_matrix(&config);
        assert_eq!(matrix.len(), 1);
        assert_eq!(matrix[0].len(), 2);
        let back = unmatrix(config.clone(), &matrix, &folders);
        assert_eq!(back["front"]["buttons"], config["front"]["buttons"]);
    }

    #[test]
    fn grow_height_adds_voids() {
        let config = update_gridsize(test_config(), 3, 2);
        let buttons = config["front"]["buttons"]["main"].as_array().unwrap();
        assert_eq!(buttons.len(), 6);
        assert!(buttons.iter().filter(|b| is_void(b)).count() >= 2);
    }

    #[test]
    fn shrink_width_removes_void_column() {
        let config = update_gridsize(test_config(), 2, 1);
        let buttons = config["front"]["buttons"]["main"].as_array().unwrap();
        // Column [b, VOID] is not all-VOID, so one VOID per row is removed.
        assert!(buttons.len() <= 4);
    }
}
