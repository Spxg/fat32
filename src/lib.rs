#[cfg(test)]
mod tests {
    extern crate winapi;

    use self::winapi::ctypes::c_void;
    use winapi::um::fileapi;
    use std::os::raw::{c_ulong, c_long};
    use std::ptr;

    const GENERIC_READ: c_ulong = 1 << 31;
    const FILE_SHARE_READ: c_ulong = 0x00000001;
    const OPEN_EXISTING: c_ulong = 3;
    const INVALID_HANDLE_VALUE: *mut c_void = 0xffffffffffffffff as *mut c_void;
    const FILE_BEGIN: c_ulong = 0;

    fn create_file_a(disk: &str) -> *mut c_void {
        unsafe {
            fileapi::CreateFileA(disk.as_ptr() as *const i8,
                                 GENERIC_READ,
                                 FILE_SHARE_READ,
                                 ptr::null_mut(),
                                 OPEN_EXISTING,
                                 0,
                                 ptr::null_mut())
        }
    }

    fn set_file_pointer(handle: *mut c_void, offset: c_long) {
        unsafe {
            fileapi::SetFilePointer(handle,
                                    offset,
                                    ptr::null_mut(),
                                    FILE_BEGIN);
        }
    }

    fn read_file(handle: *mut c_void,
                 buf: &mut [u8],
                 number_of_blocks: c_ulong,
                 number_of_bytes_read: &mut c_ulong,
    ) -> bool {
        let bool_int = unsafe {
            fileapi::ReadFile(handle,
                              buf.as_ptr() as *mut c_void,
                              number_of_blocks * 512,
                              number_of_bytes_read as *mut c_ulong,
                              ptr::null_mut())
        };

        bool_int != 0
    }

    #[test]
    fn test() {
        let disk = "\\\\.\\F:";
        let handle = create_file_a(disk);
        assert_ne!(handle, INVALID_HANDLE_VALUE);
        set_file_pointer(handle, 0);

        let mut buf = [0; 512];
        let mut len =  0;

        let read_res = read_file(handle,
                                 &mut buf,
                                 1,
                                 &mut len);
        assert!(read_res);

        println!("{:?}", &buf[0..len as usize]);
    }
}
