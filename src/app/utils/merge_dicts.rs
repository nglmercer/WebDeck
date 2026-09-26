//! Port of `app/utils/merge_dicts.py`.

use serde_json::Value;

/// Merge two JSON values, recursing into sub-objects.
///
/// Keys in `d2` overwrite `d1`, unless both sides are objects — then they are
/// merged recursively. Mirrors `merge_dicts(d1, d2)` exactly (including the
/// mutate-and-return-`d1` semantics, expressed here by taking `d1` by value).
pub fn merge_dicts(mut d1: Value, d2: &Value) -> Value {
    if let (Some(map1), Some(map2)) = (d1.as_object_mut(), d2.as_object()) {
        for (key, value2) in map2 {
            match (map1.get_mut(key), value2) {
                (Some(existing), Value::Object(_)) if existing.is_object() => {
                    let taken = std::mem::take(existing);
                    *existing = merge_dicts(taken, value2);
                }
                _ => {
                    map1.insert(key.clone(), value2.clone());
                }
            }
        }
    } else {
        d1 = d2.clone();
    }
    d1
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merges_nested_objects() {
        let d1 = json!({"a": 1, "nested": {"x": 1, "y": 2}});
        let d2 = json!({"b": 2, "nested": {"y": 20, "z": 30}});
        assert_eq!(
            merge_dicts(d1, &d2),
            json!({"a": 1, "b": 2, "nested": {"x": 1, "y": 20, "z": 30}})
        );
    }

    #[test]
    fn overwrites_non_objects() {
        let d1 = json!({"a": {"x": 1}, "b": 1});
        let d2 = json!({"a": 5, "b": {"y": 2}});
        assert_eq!(merge_dicts(d1, &d2), json!({"a": 5, "b": {"y": 2}}));
    }
}
