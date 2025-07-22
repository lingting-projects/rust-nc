pub fn is_true(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }


    match s.to_lowercase().as_ref() {
        "1" | "true" | "t" | "y" | "ok" => true,
        _ => false,
    }
}

pub fn is_false(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    match s.to_lowercase().as_ref() {
        "0" | "false" | "f" | "n" | "no" => true,
        _ => false,
    }
}
