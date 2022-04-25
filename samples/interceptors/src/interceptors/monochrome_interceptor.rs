use crate::{KeyPressRequest, KeyPressResult};
use crossterm::style::{Color, ContentStyle, StyledContent, Stylize};
use mediator::Interceptor;
use std::sync::atomic::{AtomicBool, Ordering};

static IS_EVEN: AtomicBool = AtomicBool::new(false);

pub struct MonochromeInterceptor;

impl Interceptor<KeyPressRequest, KeyPressResult> for MonochromeInterceptor {
    fn handle(
        &mut self,
        req: KeyPressRequest,
        next: Box<dyn FnOnce(KeyPressRequest) -> KeyPressResult>,
    ) -> KeyPressResult {
        let res = next(req);

        match res {
            KeyPressResult::Print(styled_content) => {
                let is_even = IS_EVEN.swap(!IS_EVEN.load(Ordering::SeqCst), Ordering::SeqCst);
                let content = styled_content.content().clone();
                let style = if is_even {
                    ContentStyle::new().with(Color::White).bold()
                } else {
                    ContentStyle::new().with(Color::DarkGrey)
                };

                let new_styled = StyledContent::new(style, content);
                KeyPressResult::Print(new_styled)
            }
            _ => res,
        }
    }
}
