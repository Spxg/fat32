use crate::base::BasicOperation;
use crate::bpb::BIOSParameterBlock;
use crate::file::File;
use crate::iter::FindIter;

#[derive(Debug)]
pub enum FileError {
    NoMatchFile
}

#[derive(Debug, Copy, Clone)]
pub struct Dir<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    pub base: BASE,
    pub bpb: BIOSParameterBlock,
    pub dir_name: [u8; 8],
    pub create_ms: u8,
    pub create_time: [u8; 2],
    pub create_date: [u8; 2],
    pub visit_date: [u8; 2],
    pub edit_time: [u8; 2],
    pub edit_date: [u8; 2],
    pub dir_cluster: u32,
    pub length: u32,
}

impl<BASE> Dir<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    pub fn file(&self, file: &str) -> Result<File<BASE>, FileError> {
        let mut buf = [0; 512];
        self.base.read(&mut buf, self.bpb.offset(self.dir_cluster), 1).unwrap();

        let index = file.find(".").unwrap();
        let file_name = &file[0..index];
        let extension_name = &file[index + 1..];

        for info in self.find(buf) {
            if info[0x0B] == 0x20 {
                let f = match core::str::from_utf8(&info[0..index]) {
                    Ok(name) => name,
                    Err(_) => { continue; }
                };

                let e = match core::str::from_utf8(&info[0x08..0x08 + extension_name.len()]) {
                    Ok(name) => name,
                    Err(_) => continue,
                };

                if file_name.eq_ignore_ascii_case(f) && extension_name.eq_ignore_ascii_case(e) {
                    return Ok(self.get_file(&info));
                }
            }
        }

        Err(FileError::NoMatchFile)
    }

    fn find(&self, buf: [u8; 512]) -> FindIter {
        FindIter {
            block: buf,
            at: 0,
        }
    }

    fn get_file(&self, buf: &[u8; 32]) -> File<BASE> {
        let mut file_name = [0; 8];
        let mut extension_name = [0; 3];
        let create_time = [buf[0x0F], buf[0x0E]];
        let create_date = [buf[0x11], buf[0x10]];
        let last_visit_date = [buf[0x13], buf[0x12]];
        let edit_time = [buf[0x17], buf[0x16]];
        let edit_date = [buf[0x19], buf[0x18]];

        for i in 0x00..0x08 {
            file_name[i] = buf[i];
        }

        for i in 0x08..0x0B {
            extension_name[i - 0x08] = buf[i];
        }

        File::<BASE> {
            base: self.base,
            bpb: self.bpb,
            file_name,
            extension_name,
            create_ms: buf[0x0D],
            create_time,
            create_date,
            visit_date: last_visit_date,
            edit_time,
            edit_date,
            file_cluster: ((buf[0x15] as u32) << 24)
                | ((buf[0x14] as u32) << 16)
                | ((buf[0x1B] as u32) << 8)
                | (buf[0x1A] as u32),
            length: ((buf[0x1F] as u32) << 24)
                | ((buf[0x1E] as u32) << 16)
                | ((buf[0x1D] as u32) << 8)
                | (buf[0x1C] as u32),
        }
    }
}