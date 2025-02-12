use cliclack::{Theme, ThemeState};
use console::Style;

pub struct DefaultTheme;

impl Theme for DefaultTheme {
    fn bar_color(&self, _: &ThemeState) -> Style {
        Style::new().dim().bold()
    }

    fn state_symbol_color(&self, _state: &ThemeState) -> Style {
        Style::new().cyan()
    }

    fn info_symbol(&self) -> String {
        "⚙".into()
    }
}
