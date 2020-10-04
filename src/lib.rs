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
    use winapi::um::winioctl;
    use winapi::um::ioapiset;
    use block_device::BlockDevice;
    use core::ptr;
    use core::str;
    use crate::volume::Volume;
    use self::winapi::ctypes::{
        c_void,
        c_ulong,
        c_long,
    };
    use crate::dir::DirError;
    use crate::BUFFER_SIZE;
    use crate::file::WriteType;

    const GENERIC_READ: c_ulong = 1 << 31;
    const GENERIC_WRITE: c_ulong = 1 << 30;
    const FILE_SHARE_READ: c_ulong = 0x00000001;
    const FILE_SHARE_WRITE: c_ulong = 0x00000002;
    const OPEN_EXISTING: c_ulong = 3;
    const INVALID_HANDLE_VALUE: *mut c_void = 0xffffffffffffffff as *mut c_void;
    const FILE_BEGIN: c_ulong = 0;

    #[derive(Debug)]
    enum DeviceError {
        ReadError,
        WriteError,
    }

    #[derive(Debug, Copy, Clone)]
    struct Device {
        handle: *mut c_void,
    }

    impl Device {
        fn mount() -> Self {
            let disk = "\\\\.\\E:";
            let handle = unsafe {
                fileapi::CreateFileA(disk.as_ptr() as *const i8,
                                     GENERIC_READ | GENERIC_WRITE,
                                     FILE_SHARE_READ | FILE_SHARE_WRITE,
                                     ptr::null_mut(),
                                     OPEN_EXISTING,
                                     0,
                                     ptr::null_mut())
            };

            assert_ne!(handle, INVALID_HANDLE_VALUE);

            let _lp = 0;
            let code = unsafe {
                ioapiset::DeviceIoControl(handle,
                                          winioctl::FSCTL_DISMOUNT_VOLUME,
                                          ptr::null_mut(),
                                          0,
                                          ptr::null_mut(),
                                          0,
                                          _lp as *mut c_ulong,
                                          ptr::null_mut())
            };

            assert_eq!(code, 1);

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

        fn _read(&self,
                 buf: &mut [u8],
                 number_of_blocks: usize,
                 number_of_bytes_read: &mut c_ulong,
        ) -> bool {
            let bool_int = unsafe {
                fileapi::ReadFile(self.handle,
                                  buf.as_ptr() as *mut c_void,
                                  (BUFFER_SIZE * number_of_blocks) as c_ulong,
                                  number_of_bytes_read as *mut c_ulong,
                                  ptr::null_mut())
            };

            bool_int != 0
        }

        fn _write(&self,
                  buf: &[u8],
                  number_of_blocks: usize,
                  number_of_bytes_write: &mut c_ulong,
        ) -> bool {
            let bool_int = unsafe {
                fileapi::WriteFile(self.handle,
                                   buf.as_ptr() as *const c_void,
                                   (BUFFER_SIZE * number_of_blocks) as c_ulong,
                                   number_of_bytes_write as *mut c_ulong,
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
            let res = self._read(buf, number_of_blocks, &mut len);
            if res { Ok(()) } else { Err(DeviceError::ReadError) }
        }

        fn write(&self, buf: &[u8], address: usize, number_of_blocks: usize) -> Result<(), Self::Error> {
            let mut len = 0;
            self.set_file_pointer(address as i32);
            let res = self._write(buf, number_of_blocks, &mut len);
            if res { Ok(()) } else { Err(DeviceError::WriteError) }
        }
    }

    #[test]
    fn test_all() {
        let device = Device::mount();
        let volume = Volume::new(device);
        let mut root = volume.root_dir();
        let mut buf = [0; 20480];

        let test_dir = root.create_dir("test_dir");
        assert!(test_dir.is_ok());

        let test_dir = root.cd("test_dir");
        assert!(test_dir.is_ok());
        let mut test_dir = test_dir.unwrap();

        let illegal_char = test_dir.create_file("illegal_char:");
        assert_eq!(illegal_char.err().unwrap(), DirError::IllegalChar);

        let lfn_file = test_dir.create_file("Rust牛逼.txt");
        assert!(lfn_file.is_ok());

        let file = test_dir.open_file("Rust牛逼.txt");
        assert!(file.is_ok());

        let mut file = file.unwrap();
        file.write("停留牛逼，测试一把梭".as_bytes(), WriteType::OverWritten).unwrap();
        let length = file.read(&mut buf);
        assert!(length.is_ok());
        assert_eq!("停留牛逼，测试一把梭", str::from_utf8(&buf[0..length.unwrap()]).unwrap());

        file.write(&[48; 10240], WriteType::Append).unwrap();
        let length = file.read(&mut buf);
        assert!(length.is_ok());
        assert_eq!([48; 10240], buf["停留牛逼，测试一把梭".len()..length.unwrap()]);

        file.write(&[48; 10241], WriteType::OverWritten).unwrap();
        let length = file.read(&mut buf);
        assert!(length.is_ok());
        assert_eq!([48; 10241], buf[0..length.unwrap()]);

        let delete_dir = test_dir.delete_dir("Rust牛逼.txt");
        assert_eq!(delete_dir.err().unwrap(), DirError::NoMatchDir);
        let delete_file = test_dir.delete_file("Rust牛逼.txt");
        assert!(delete_file.is_ok());
        let delete_test_dir = root.delete_dir("test_dir");
        assert!(delete_test_dir.is_ok());
    }
}
