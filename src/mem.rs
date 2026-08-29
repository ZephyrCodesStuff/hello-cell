//! C runtime memory functions required by compiler builtins.

use core::ffi::c_void;

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let d = dest as *mut u8;
    let s = src as *const u8;
    let mut i = 0;
    while i < n {
        *d.add(i) = *s.add(i);
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut c_void, c: i32, n: usize) -> *mut c_void {
    let d = dest as *mut u8;
    let mut i = 0;
    while i < n {
        *d.add(i) = c as u8;
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> i32 {
    let a = s1 as *const u8;
    let b = s2 as *const u8;
    let mut i = 0;
    while i < n {
        let va = *a.add(i);
        let vb = *b.add(i);
        if va != vb {
            return (va as i32) - (vb as i32);
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let d = dest as *mut u8;
    let s = src as *const u8;
    if (d as usize) < (s as usize) {
        let mut i = 0;
        while i < n {
            *d.add(i) = *s.add(i);
            i += 1;
        }
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            *d.add(i) = *s.add(i);
        }
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn strlen(s: *const u8) -> usize {
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}
