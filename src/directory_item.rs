use crate::tool::read_le_u32;
use core::str;

#[derive(Copy, Clone, Debug)]
pub enum ItemType {
    Dir,
    File,
    LFN,
}

impl ItemType {
    fn from_value(value: u8) -> ItemType {
        if (value & 0x10) == 0x10 {
            ItemType::Dir
        } else if value == 0x0F {
            ItemType::LFN
        } else {
            ItemType::File
        }
    }
}

impl Default for ItemType {
    fn default() -> Self {
        ItemType::Dir
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct ShortDirectoryItem {
    name: [u8; 8],
    extension: [u8; 3],
    create_ms: u8,
    create_time: [u8; 2],
    create_date: [u8; 2],
    last_visit_date: [u8; 2],
    edit_time: [u8; 2],
    edit_date: [u8; 2],
    length: u32,
    cluster: u32,
}

impl ShortDirectoryItem {
    fn root_dir(cluster: u32) -> _DirectoryItem {
        let s = Self {
            cluster,
            ..Self::default()
        };
        _DirectoryItem::from_short(s)
    }

    pub fn from_buf(buf: &[u8]) -> _DirectoryItem {
        let mut name = [0; 8];
        let mut extension = [0; 3];
        let create_time = [buf[0x0F], buf[0x0E]];
        let create_date = [buf[0x11], buf[0x10]];
        let last_visit_date = [buf[0x13], buf[0x12]];
        let edit_time = [buf[0x17], buf[0x16]];
        let edit_date = [buf[0x19], buf[0x18]];

        name.copy_from_slice(&buf[0x00..0x08]);
        extension.copy_from_slice(&buf[0x08..0x0b]);

        let s = Self {
            name,
            extension,
            create_ms: buf[0x0D],
            create_time,
            create_date,
            last_visit_date,
            edit_time,
            edit_date,
            cluster: ((buf[0x15] as u32) << 24)
                | ((buf[0x14] as u32) << 16)
                | ((buf[0x1B] as u32) << 8)
                | (buf[0x1A] as u32),
            length: read_le_u32(&buf[0x1C..0x20]),
        };

        _DirectoryItem::from_short(s)
    }

    fn get_full_name_bytes(&self) -> ([u8; 12], usize) {
        let mut len = 0;
        let mut full_name = [0; 12];

        for &i in self.name.iter() {
            if i != 0x20 {
                full_name[len] = i;
                len += 1;
            }
        }

        if self.extension[0] != 0x20 {
            full_name[len] = b'.';
            len += 1;
        }

        for &i in self.extension.iter() {
            if i != 0x20 {
                full_name[len] = i;
                len += 1;
            }
        }

        (full_name, len)
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct LongDirectoryItem {
    attribute: u8,
    unicode_part1: [u8; 10],
    unicode_part2: [u8; 12],
    unicode_part3: [u8; 4],
}

impl LongDirectoryItem {
    fn from_buf(buf: &[u8]) -> _DirectoryItem {
        let attribute = buf[0x00];
        let mut unicode_part1 = [0; 10];
        let mut unicode_part2 = [0; 12];
        let mut unicode_part3 = [0; 4];

        unicode_part1.copy_from_slice(&buf[0x01..0x0B]);
        unicode_part2.copy_from_slice(&buf[0x0E..0x1A]);
        unicode_part3.copy_from_slice(&buf[0x1C..0x20]);

        let l = Self {
            attribute,
            unicode_part1,
            unicode_part2,
            unicode_part3,
        };

        _DirectoryItem::from_long(l)
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct _DirectoryItem {
    short: Option<ShortDirectoryItem>,
    long: Option<LongDirectoryItem>,
}

impl _DirectoryItem {
    pub fn cluster(&self) -> u32 {
        self.short.unwrap().cluster
    }

    fn from_short(value: ShortDirectoryItem) -> Self {
        Self {
            short: Some(value),
            long: None,
        }
    }

    fn from_long(value: LongDirectoryItem) -> Self {
        Self {
            short: None,
            long: Some(value),
        }
    }

    fn get_full_name_bytes(&self) -> ([u8; 12], usize) {
        if self.short.is_some() {
            self.short.unwrap().get_full_name_bytes()
        } else {
            ([0; 12], 0)
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct DirectoryItem {
    pub(crate) item_type: ItemType,
    pub(crate) item: _DirectoryItem,
}

impl DirectoryItem {
    pub fn root_dir(cluster: u32) -> Self {
        Self {
            item: ShortDirectoryItem::root_dir(cluster),
            ..Self::default()
        }
    }

    pub fn from_buf(buf: &[u8]) -> Self {
        let item_type = ItemType::from_value(buf[0x0B]);

        let item = match item_type {
            ItemType::LFN => LongDirectoryItem::from_buf(buf),
            _ => ShortDirectoryItem::from_buf(buf)
        };

        Self {
            item_type,
            item,
        }
    }

    pub fn equal(&self, value: &str) -> bool {
        let (bytes, len) = self.item.get_full_name_bytes();
        if let Ok(res) = str::from_utf8(&bytes[0..len]) {
            value.eq_ignore_ascii_case(res)
        } else {
            false
        }
    }
}