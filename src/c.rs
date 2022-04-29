//! Internal helpers to interface with the c ffmpeg code

use std::path::Path;
use std::{cmp, ffi, ptr};

#[repr(C)]
pub struct Stream {
    pub length: usize,
    pub offset: usize,
    pub data: *const u8,
}

pub unsafe extern "C" fn read_stream(ptr: *mut ffi::c_void, buf: *mut u8, size: i32) -> i32 {
    let stream = &mut *(ptr as *mut Stream);
    let size = cmp::min(size as usize, stream.length - stream.offset);

    ptr::copy_nonoverlapping(stream.data.add(stream.offset), buf, size);
    stream.offset += size;

    size as i32
}

pub fn path_to_raw(path: &Path) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    buf.extend(path.to_str()?.as_bytes());
    buf.push(0);
    Some(buf)
}
