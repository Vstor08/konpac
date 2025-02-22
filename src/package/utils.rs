extern crate libc;

pub fn is_elevated() -> bool {
    unsafe { libc::getuid() == 0 }
}