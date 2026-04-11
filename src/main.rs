#![windows_subsystem = "windows"]
pub use cosmic::app::Settings;
pub use mctest::gui::App;
fn main() {
    let settings = Settings::default().exit_on_close(true);
    cosmic::app::run::<App>(settings, ()).unwrap();
}
