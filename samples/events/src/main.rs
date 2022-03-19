use std::io;
use colored::Colorize;
use mediator::{DefaultMediator, Event, Mediator};

#[derive(Clone)]
struct UpperTextEvent(String);
impl Event for UpperTextEvent {}

#[derive(Clone)]
struct ReverseTextEvent(String);
impl Event for ReverseTextEvent {}

fn main() {
    let mut mediator = DefaultMediator::builder()
        .subscribe_fn(|event: UpperTextEvent| {
            let text = event.0.trim().to_uppercase();
            println!("{:<15}{}", "Upper:".bright_white(), text.as_str().blue());
        })
        .subscribe_fn(|event: ReverseTextEvent| {
            let text = event.0.trim().chars().rev().collect::<String>();
            println!("{:<15}{}", "Reversed:".bright_white(), text.as_str().red());
        })
        .build();

    loop {
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).expect("Failed to read line");

        if buf.trim() == "exit" {
            break;
        }

        mediator.publish(UpperTextEvent(buf.clone())).expect("Failed to publish event");
        mediator.publish(ReverseTextEvent(buf)).expect("Failed to publish event");
    }
}
