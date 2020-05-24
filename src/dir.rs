use crate::base::BasicOperation;
use crate::bpb::BIOSParameterBlock;
use crate::file::File;
use crate::iter::{FindIter, FindRevIter};

#[derive(Debug)]
pub enum DirError {
    NoMatch
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
    pub fn file(&self, file: &str) -> Result<File<BASE>, DirError> {
        match self.find(file) {
            Ok(buf) => {
                return Ok(self.get_file(&buf));
            }
            Err(_) => {
                Err(DirError::NoMatch)
            }
        }
    }

    fn find(&self, name: &str) -> Result<[u8; 32], DirError> {
        let mut file = name;

        let mut buf = [0; 512];
        let mut offset_count = 0;
        let mut at = 0;

        loop {
            self.base.read(&mut buf, self.bpb.offset(self.dir_cluster) + offset_count * 512, 1).unwrap();

            if buf[0x00] == 0x00 {
                break;
            }

            for info in self.find_buf(buf, at) {
                if info.1 >= 512 {
                    at = info.1 - 512;
                    break;
                }

                if info.0[0x0B] == 0x0F {
                    let count = info.0[0x00] & 0x1F;
                    let start_at = info.1 + (count + 1) as usize * 32;
                    let mut temp = [0; 32];

                    for name in (FindRevIter { block: buf, at: start_at, end: info.1 }) {
                        if name.1 == start_at {
                            temp = name.0;
                            continue;
                        }

                        let part = self.get_long_name(&name.0);
                        let part_name = core::str::from_utf8(&part.0[0..part.1]).unwrap();

                        if part.1 > file.len() {
                            break;
                        }

                        if part_name.eq(&file[0..part.1]) {
                            file = &file[part.1..];
                        } else {
                            break;
                        }

                        if file.len() == 0 {
                            return Ok(temp);
                        }
                    }
                } else {
                    let file_name = self.get_short_name(&info.0);
                    if let Ok(file_name) = core::str::from_utf8(&file_name.0[0..file_name.1]) {
                        if file.eq_ignore_ascii_case(file_name) {
                            return Ok(info.0);
                        }
                    }
                }
            }
            offset_count += 1;
        }

        Err(DirError::NoMatch)
    }

    fn find_buf(&self, buf: [u8; 512], at: usize) -> FindIter {
        FindIter {
            block: buf,
            at,
        }
    }

    fn get_file(&self, buf: &[u8]) -> File<BASE> {
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

    fn get_short_name(&self, buf: &[u8; 32]) -> ([u8; 13], usize) {
        let mut file_name = [0; 13];
        let mut index = 0;

        for i in 0x00..=0x0A {
            if buf[i] != 0x20 {
                if i == 0x08 {
                    file_name[index] = '.' as u8;
                    index += 1;
                }
                file_name[index] = buf[i];
                index += 1;
            }
        }

        (file_name, index)
    }

    fn get_long_name(&self, buf: &[u8; 32]) -> ([u8; 13 * 3], usize) {
        let mut res = ([0; 13 * 3], 0);

        let op = |res: &mut ([u8; 13 * 3], usize), start: usize, end: usize| {
            for i in (start..end).step_by(2) {
                if buf[i] == 0x00 {
                    break;
                }

                let unicode = (((buf[i + 1] as u16) << 8) as u16) | buf[i] as u16;

                if unicode <= 0x007F {
                    res.0[res.1] = unicode as u8;
                    res.1 += 1;
                } else if unicode >= 0x0080 && unicode <= 0x07FF {
                    let part1 = (0b11000000 | (0b00011111 & (unicode >> 6))) as u8;
                    let part2 = (0b10000000 | (0b00111111) & unicode) as u8;

                    res.0[res.1] = part1;
                    res.0[res.1 + 1] = part2;
                    res.1 += 2;
                } else if unicode >= 0x0800 {
                    let part1 = (0b11100000 | (0b00011111 & (unicode >> 12))) as u8;
                    let part2 = (0b10000000 | (0b00111111) & (unicode >> 6)) as u8;
                    let part3 = (0b10000000 | (0b00111111) & unicode) as u8;

                    res.0[res.1] = part1;
                    res.0[res.1 + 1] = part2;
                    res.0[res.1 + 2] = part3;
                    res.1 += 3;
                }
            }
        };

        if buf[0x01] != 0xFF {
            op(&mut res, 0x01, 0x0A);
        }

        if buf[0x0E] != 0xFF {
            op(&mut res, 0x0E, 0x19);
        }

        if buf[0x1C] != 0xFF {
            op(&mut res, 0x1C, 0x1F);
        }

        return res;
    }
}