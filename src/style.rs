use std::io::{self, IsTerminal};
use std::sync::OnceLock;

static ENABLED: OnceLock<bool> = OnceLock::new();

pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| io::stdout().is_terminal() && !no_color())
}

fn no_color() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
}

pub fn dim(text: &str) -> String {
    if enabled() {
        format!("\x1b[90m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}
