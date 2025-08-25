use std::ffi::OsStr;

pub fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(k: K, v: V) {
    unsafe {
        std::env::set_var(k, v);
    }
}

pub fn remove_var<K: AsRef<OsStr>>(k: K) {
    unsafe {
        std::env::remove_var(k);
    }
}
