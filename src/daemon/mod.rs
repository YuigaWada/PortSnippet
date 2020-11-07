#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::register;
pub use self::macos::run;
pub use self::macos::stop;
pub use self::macos::get_complete_messages;