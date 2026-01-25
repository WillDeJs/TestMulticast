pub use cosmic::app::Settings;
pub use mctest::gui::App;
fn main() {
    let settings = Settings::default();
    cosmic::app::run::<App>(settings, ()).unwrap();
}
