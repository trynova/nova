use cliclack::{Theme, ThemeState};
use console::Style;

pub struct DefaultTheme;

impl Theme for DefaultTheme {
    fn bar_color(&self, _: &ThemeState) -> Style {
        Style::new().dim().bold()
    }

    fn state_symbol_color(&self, _: &ThemeState) -> Style {
        Style::new().cyan()
    }

    fn input_style(&self, _: &ThemeState) -> Style {
        Style::new().yellow()
    }

    fn format_intro(&self, title: &str) -> String {
        let color = self.bar_color(&ThemeState::Submit);
        format!(
            "{start_bar}  {title} {exit_instructions}\n{bar}\n",
            start_bar = color.apply_to("⚙"),
            bar = color.apply_to("|"),
            title = Style::new().bold().apply_to(title),
            exit_instructions = color.apply_to("(type exit or Ctrl+C to exit)"),
        )
    }
}
