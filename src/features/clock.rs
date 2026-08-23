use chrono::Local;

pub fn current() -> String {
    Local::now().format("%H:%M").to_string()
}
