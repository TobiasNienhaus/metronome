use fltk::{app, prelude::*, window::Window};
use fltk::button::Button;
use fltk::group::Flex;
use fltk_theme::{ColorTheme, color_themes};
use fltk_theme::{widget_schemes, WidgetTheme, ThemeType};
use fltk_theme::{WidgetScheme, SchemeType};

mod gui_v1;

fn main() {
    let a = app::App::default().with_scheme(app::Scheme::Base);

    let wt = WidgetTheme::new(ThemeType::Greybird);
    let ct = ColorTheme::new(color_themes::BLACK_THEME);
    let ws = WidgetScheme::new(SchemeType::Fluent);
    wt.apply();
    ct.apply();
    ws.apply();

    let mut ui = gui_v1::UserInterface::make_window();
    
    a.run().unwrap();
}
