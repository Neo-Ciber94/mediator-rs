use crossterm::style::StyledContent;
use std::fmt::Display;
use std::io::Write;

pub fn flush() {
    std::io::stdout().flush().unwrap();
}

pub fn print_styled<S: Display>(styled: StyledContent<S>) {
    print!("{}", styled);
    flush();
}
