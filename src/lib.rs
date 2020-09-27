#![no_std]

pub mod bpb;
pub mod volume;
pub mod tool;
pub mod dir;
pub mod directory_item;
pub mod file;
pub mod fat;

#[macro_use]
extern crate std;

const BUFFER_SIZE: usize = 512;

#[cfg(test)]
mod fat32 {
    extern crate winapi;

    use winapi::um::fileapi;
    use block_device::BlockDevice;
    use core::ptr;
    use core::str;
    use crate::volume::Volume;
    use self::winapi::ctypes::{c_void, c_ulong, c_long};
    use crate::dir::DirError;
    use crate::BUFFER_SIZE;

    const GENERIC_READ: c_ulong = 1 << 31;
    const FILE_SHARE_READ: c_ulong = 0x00000001;
    const OPEN_EXISTING: c_ulong = 3;
    const INVALID_HANDLE_VALUE: *mut c_void = 0xffffffffffffffff as *mut c_void;
    const FILE_BEGIN: c_ulong = 0;

    #[derive(Debug)]
    enum DeviceError {
        ReadError
    }

    #[derive(Debug, Copy, Clone)]
    struct Device {
        handle: *mut c_void,
    }

    impl Device {
        fn mount_read() -> Self {
            let disk = "\\\\.\\F:";
            let handle = unsafe {
                fileapi::CreateFileA(disk.as_ptr() as *const i8,
                                     GENERIC_READ,
                                     FILE_SHARE_READ,
                                     ptr::null_mut(),
                                     OPEN_EXISTING,
                                     0,
                                     ptr::null_mut())
            };

            assert_ne!(handle, INVALID_HANDLE_VALUE);

            Self {
                handle
            }
        }

        fn set_file_pointer(&self, offset: c_long) {
            unsafe {
                fileapi::SetFilePointer(self.handle,
                                        offset,
                                        ptr::null_mut(),
                                        FILE_BEGIN);
            }
        }

        fn read(&self,
                buf: &mut [u8],
                number_of_blocks: c_ulong,
                number_of_bytes_read: &mut c_ulong,
        ) -> bool {
            let bool_int = unsafe {
                fileapi::ReadFile(self.handle,
                                  buf.as_ptr() as *mut c_void,
                                  number_of_blocks * 512,
                                  number_of_bytes_read as *mut c_ulong,
                                  ptr::null_mut())
            };

            bool_int != 0
        }
    }

    impl BlockDevice for Device {
        type Error = DeviceError;

        fn read(&self, buf: &mut [u8], address: usize, number_of_blocks: usize) -> Result<(), Self::Error> {
            let mut len = 0;
            self.set_file_pointer(address as i32);
            let res = self.read(buf, number_of_blocks as c_ulong, &mut len);
            if res { Ok(()) } else { Err(DeviceError::ReadError) }
        }

        fn write(&self, _buf: &[u8], _address: usize, _number_of_blocks: usize) -> Result<(), Self::Error> {
            unimplemented!()
        }
    }

    #[test]
    fn test_all() {
        let device = Device::mount_read();
        let volume = Volume::new(device);
        let root = volume.root_dir();

        let dir = root.into_dir("这是一个测试-Rust");
        assert!(dir.is_ok());
        let dir = dir.unwrap();

        let exist = dir.exist("Rust牛逼.txt");
        assert!(exist.is_some());
        let exist = dir.exist("cnb.txt");
        assert!(exist.is_some());

        let cnb = dir.open_file("cnb.txt");
        assert!(cnb.is_ok());
        let cnb = cnb.unwrap();
        let mut buf = [0; BUFFER_SIZE];
        let read = cnb.read(&mut buf);
        assert!(read.is_ok());
        let length = read.unwrap();
        assert_eq!("c牛逼", str::from_utf8(&buf[0..length]).unwrap());

        let not_exist = root.into_dir("not_exist_dir");
        assert_eq!(DirError::NoMatchDir, not_exist.err().unwrap());
    }
}
