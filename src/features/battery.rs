use std::fs;
use std::path::Path;

pub fn percentage() -> String {
    let base = Path::new("/sys/class/power_supply");

    let Ok(entries) = fs::read_dir(base) else {
        return "--".to_string();
    };

    for entry in entries.flatten() {
        let path = entry.path();

        let Ok(capacity) = fs::read_to_string(path.join("capacity")) else {
            continue;
        };

        if let Ok(value) = capacity.trim().parse::<u32>() {
            return format!("{value}%");
        }
    }

    "--".to_string()
}

pub fn icon() -> String {
    let base = Path::new("/sys/class/power_supply");

    let Ok(entries) = fs::read_dir(base) else {
        return "󰂃".to_string();
    };

    for entry in entries.flatten() {
        let path = entry.path();

        let Ok(capacity) = fs::read_to_string(path.join("capacity")) else {
            continue;
        };

        let Ok(value) = capacity.trim().parse::<u32>() else {
            continue;
        };

        let Ok(status) = fs::read_to_string(path.join("status")) else {
            return battery_icon(value);
        };

        if status.trim().eq_ignore_ascii_case("charging") {
            return "󰂄".to_string();
        }

        if status.trim().eq_ignore_ascii_case("full") {
            return "󰁹".to_string();
        }

        return battery_icon(value);
    }

    "󰂃".to_string()
}

fn battery_icon(value: u32) -> String {
    match value {
        0..=10 => "󰁺".to_string(),
        11..=20 => "󰁻".to_string(),
        21..=30 => "󰁼".to_string(),
        31..=40 => "󰁽".to_string(),
        41..=50 => "󰁾".to_string(),
        51..=60 => "󰁿".to_string(),
        61..=70 => "󰂀".to_string(),
        71..=80 => "󰂁".to_string(),
        81..=90 => "󰂂".to_string(),
        _ => "󰁹".to_string(),
    }
}
