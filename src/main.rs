#![windows_subsystem = "windows"]
pub use cosmic::app::Settings;
pub use mctest::gui::App;
fn main() {
    let settings = Settings::default().exit_on_close(false);
    cosmic::app::run::<App>(settings, ()).unwrap();
}
