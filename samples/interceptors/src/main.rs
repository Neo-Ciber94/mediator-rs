mod ext;
mod interceptors;
mod macros;
mod requests;
mod utils;

use crate::ext::ObjectExt;
use crate::interceptors::colored_interceptor::ColoredInterceptor;
use crate::interceptors::monochrome_interceptor::MonochromeInterceptor;
use crate::interceptors::upper_case_interceptor::UpperCaseInterceptor;
use crate::requests::key_press::{KeyPressRequest, KeyPressRequestHandler, KeyPressResult};
use crate::utils::{flush, print_styled};
use crossterm::event::{read, Event};
use crossterm::style::{Color, ContentStyle, StyledContent, Stylize};
use mediator::{DefaultMediator, Mediator};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub const EXIT_COMMAND: &str = "exit";

fn main() -> crossterm::Result<()> {
    fn print_exit_message() {
        print_styled(StyledContent::new(
            ContentStyle::new().with(Color::DarkRed),
            "\nExiting...\n",
        ))
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))
        .expect("Error setting Ctrl-C handler");

    let mut mediator = create_mediator();
    let buf: Arc<RwLock<String>> = Default::default();

    loop {
        if !running.load(Ordering::SeqCst) {
            print_exit_message();
            break;
        }

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = read()? {
                match mediator.send(KeyPressRequest(buf.clone(), key)).unwrap() {
                    KeyPressResult::Exit => {
                        println!();
                        print_exit_message();
                        break;
                    }
                    KeyPressResult::Continue => {}
                    KeyPressResult::Print(s) => {
                        print!("{}", s);
                    }
                }
            }
        }

        // Flush to print out the content of the buffer
        flush();
    }

    Ok(())
}

fn create_mediator() -> DefaultMediator {
    DefaultMediator::builder()
        .add_handler(KeyPressRequestHandler)
        .also_if(cfg!(feature = "colored"), |m| {
            m.add_interceptor(ColoredInterceptor)
        })
        .also_if(cfg!(feature = "uppercase"), |m| {
            m.add_interceptor(UpperCaseInterceptor)
        })
        .also_if(cfg!(feature = "monochrome"), |m| {
            m.add_interceptor(MonochromeInterceptor)
        })
        .build()
}
