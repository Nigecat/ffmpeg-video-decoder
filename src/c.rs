//! Internal helpers to interface with the c ffmpeg code

use std::ffi::{self, CString};
use std::path::Path;
use std::{cmp, ptr};

#[repr(C)]
pub struct Stream {
    length: usize,
    offset: usize,
    data: *const u8,
}

pub unsafe extern "C" fn read_stream(ptr: *mut ffi::c_void, buf: *mut u8, size: i32) -> i32 {
    let stream = &mut *(ptr as *mut Stream);
    let size = cmp::min(size as usize, stream.length - stream.offset);

    ptr::copy_nonoverlapping(stream.data.add(stream.offset), buf, size);
    stream.offset += size;

    size as i32
}

pub fn path_to_cstring(path: &Path) -> CString {
    unimplemented!();

    // let mut buf = Vec::new();

    // #[cfg(unix)]
    // {
    //     use std::os::unix::ffi::OsStrExt;
    //     buf.extend(path.as_os_str().as_bytes());
    //     buf.push(0);
    // }

    // #[cfg(windows)]
    // {
    //     use std::os::windows::ffi::OsStrExt;
    //     buf.extend(
    //         path.as_os_str()
    //             .encode_wide()
    //             .chain(Some(0))
    //             .map(|b| {
    //                 let b = b.to_ne_bytes();
    //                 b.get(0).map(|s| *s).into_iter().chain(b.get(1).map(|s| *s))
    //             })
    //             .flatten(),
    //     );
    // }

    // CString::new(buf).unwrap()
}
