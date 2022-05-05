/// This macros allows to hide panicing messages in output binary when feature `no-msgs` is present.
macro_rules! expect {
    ($val:expr, $msg:expr) => {
        if cfg!(feature = "no-msgs") {
            $val.unwrap()
        } else {
            $val.expect($msg)
        }
    };
}

macro_rules! panic_msg {
    ($($t:tt)*) => {
        if cfg!(feature = "no-msgs") {
            unimplemented!()
        } else {
            panic!($($t)*)
        }
    };
}

/// Creates zero terminated string.
macro_rules! pc_str {
    ($cstr:expr) => {
        windows::core::PCSTR(concat!($cstr, "\x00").as_ptr() as _)
    };
}

mod app;
pub use app::*;

mod backup;
mod input;
mod mesh;
mod shader;
mod texture;

pub use input::InputResult;
