pub mod adb;
pub mod adb_usb;
pub mod bootimg;
pub mod fastboot;
pub mod fastboot_usb;
pub mod log;
pub mod mediatek;
mod util;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
