use crate::EXIT_COMMAND;
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::style::{ContentStyle, StyledContent};
use mediator::{Request, RequestHandler};
use std::sync::{Arc, RwLock};

pub enum KeyPressResult {
    Exit,
    Continue,
    Print(StyledContent<String>),
}

impl KeyPressResult {
    pub fn print<S: Into<String>>(s: S) -> Self {
        let styled = StyledContent::new(ContentStyle::new(), s.into());
        KeyPressResult::Print(styled)
    }

    pub fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(&String) -> String,
    {
        match self {
            KeyPressResult::Print(styled_content) => {
                let content = styled_content.content();
                let style = styled_content.style().clone();
                KeyPressResult::Print(StyledContent::new(style, f(content)))
            }
            _ => self,
        }
    }
}

pub struct KeyPressRequest(pub Arc<RwLock<String>>, pub KeyEvent);
impl Request<KeyPressResult> for KeyPressRequest {}

#[derive(Default)]
pub struct KeyPressRequestHandler;
impl RequestHandler<KeyPressRequest, KeyPressResult> for KeyPressRequestHandler {
    fn handle(&mut self, req: KeyPressRequest) -> KeyPressResult {
        let KeyPressRequest(lock, k) = req;
        let mut s = lock.write().unwrap();

        match k.code {
            KeyCode::Backspace => {
                if s.len() > 0 {
                    s.pop();
                    return KeyPressResult::print("\x1b[1D \x1b[1D");
                }
            }
            KeyCode::Enter => {
                if s.as_str() == EXIT_COMMAND {
                    return KeyPressResult::Exit;
                }
                s.clear();
                return KeyPressResult::print("\n");
            }
            KeyCode::Char(c) => {
                s.push(c);
                return KeyPressResult::print(c.to_string());
            }
            _ => {}
        }

        KeyPressResult::Continue
    }
}
