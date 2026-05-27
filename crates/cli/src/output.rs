use serde::Serialize;

pub fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Failed to serialize output: {e}"),
    }
}

#[allow(dead_code)]
pub fn print_human(_value: &impl Serialize) {
    // Human output is handled by individual commands
}
