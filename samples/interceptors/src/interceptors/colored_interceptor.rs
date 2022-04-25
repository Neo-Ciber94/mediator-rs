use crate::{KeyPressRequest, KeyPressResult};
use crossterm::style::{Color, ContentStyle, StyledContent, Stylize};
use mediator::Interceptor;
use std::sync::atomic::AtomicUsize;

static CUR_COLOR: AtomicUsize = AtomicUsize::new(0);
static COLORS: [Color; 6] = [
    Color::Red,
    Color::Green,
    Color::Blue,
    Color::Yellow,
    Color::Magenta,
    Color::Cyan,
];

pub struct ColoredInterceptor;
impl Interceptor<KeyPressRequest, KeyPressResult> for ColoredInterceptor {
    fn handle(
        &mut self,
        req: KeyPressRequest,
        next: Box<dyn FnOnce(KeyPressRequest) -> KeyPressResult>,
    ) -> KeyPressResult {
        let res = next(req);
        match res {
            KeyPressResult::Print(styled) => {
                let color_index =
                    CUR_COLOR.fetch_add(1, std::sync::atomic::Ordering::SeqCst) % COLORS.len();
                let color = COLORS[color_index];

                if color_index > COLORS.len() {
                    CUR_COLOR.store(0, std::sync::atomic::Ordering::SeqCst);
                }

                let content_style = ContentStyle::new().with(color);
                let styled = StyledContent::new(content_style, styled.content().to_string());
                KeyPressResult::Print(styled)
            }
            _ => res,
        }
    }
}
