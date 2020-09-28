use core::str;
use crate::tool::read_le_u32;
use crate::dir::CreateType;

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub enum ItemType {
    Dir,
    File,
    LFN,
    Deleted,
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

    fn from_create(value: CreateType) -> ItemType {
        match value {
            CreateType::Dir => ItemType::Dir,
            CreateType::File => ItemType::File
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
    fn new(cluster: u32, value: &str, create_type: CreateType) -> Self {
        let (name, extension) = match value.find('.') {
            Some(i) => (&value[0..i], &value[i + 1..]),
            None => (&value[0..], "")
        };

        let mut item = [0; 32];
        let _item = [0x20; 11];
        item[0x00..0x0B].copy_from_slice(&_item);
        item[0x00..0x00 + name.len()].copy_from_slice(name.as_bytes());
        item[0x08..0x08 + extension.len()].copy_from_slice(extension.as_bytes());
        item[0x00..0x00 + name.len()].make_ascii_uppercase();
        item[0x08..0x08 + extension.len()].make_ascii_uppercase();

        let mut cluster: [u8; 4] = cluster.to_be_bytes();
        cluster.reverse();

        item[0x14..0x16].copy_from_slice(&cluster[2..4]);
        item[0x1A..0x1C].copy_from_slice(&cluster[0..2]);

        match create_type {
            CreateType::Dir => item[0x0B] = 0x10,
            CreateType::File => item[0x10] = 0x20,
        }

        ShortDirectoryItem::from_buf(&item)
    }

    fn root_dir(cluster: u32) -> Self {
        Self {
            cluster,
            ..Self::default()
        }
    }

    fn from_buf(buf: &[u8]) -> Self {
        let mut name = [0; 8];
        let mut extension = [0; 3];
        let create_time = [buf[0x0F], buf[0x0E]];
        let create_date = [buf[0x11], buf[0x10]];
        let last_visit_date = [buf[0x13], buf[0x12]];
        let edit_time = [buf[0x17], buf[0x16]];
        let edit_date = [buf[0x19], buf[0x18]];

        name.copy_from_slice(&buf[0x00..0x08]);
        extension.copy_from_slice(&buf[0x08..0x0B]);

        Self {
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
        }
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

    fn bytes(&self, item_type: ItemType) -> [u8; 32] {
        let mut item = [0; 32];

        item[0x00..0x08].copy_from_slice(&self.name);
        item[0x08..0x0B].copy_from_slice(&self.extension);

        let mut cluster: [u8; 4] = self.cluster.to_be_bytes();
        cluster.reverse();

        item[0x14..0x16].copy_from_slice(&cluster[2..4]);
        item[0x1A..0x1C].copy_from_slice(&cluster[0..2]);
        item[0x0C] = 0x18;

        match item_type {
            ItemType::Dir => item[0x0B] = 0x10,
            ItemType::File => item[0x0B] = 0x20,
            ItemType::LFN => item[0x0B] = 0x0F,
            ItemType::Deleted => item[0x00] = 0xE5
        }

        item
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
    fn from_buf(buf: &[u8]) -> Self {
        let attribute = buf[0x00];
        let mut unicode_part1 = [0; 10];
        let mut unicode_part2 = [0; 12];
        let mut unicode_part3 = [0; 4];

        unicode_part1.copy_from_slice(&buf[0x01..0x0B]);
        unicode_part2.copy_from_slice(&buf[0x0E..0x1A]);
        unicode_part3.copy_from_slice(&buf[0x1C..0x20]);

        Self {
            attribute,
            unicode_part1,
            unicode_part2,
            unicode_part3,
        }
    }

    fn to_utf8(&self) -> ([u8; 13 * 3], usize) {
        let (mut utf8, mut len) = ([0; 13 * 3], 0);

        let mut op = |part: &[u8]| {
            for i in (0..part.len()).step_by(2) {
                if (part[i] == 0x00 && part[i + 1] == 0x00) || part[i] == 0xFF { break; }
                let unicode = ((part[i + 1] as u16) << 8) | part[i] as u16;

                if unicode <= 0x007F {
                    utf8[len] = unicode as u8;
                    len += 1;
                } else if unicode >= 0x0080 && unicode <= 0x07FF {
                    let part1 = (0b11000000 | (0b00011111 & (unicode >> 6))) as u8;
                    let part2 = (0b10000000 | (0b00111111) & unicode) as u8;

                    utf8[len] = part1;
                    utf8[len + 1] = part2;
                    len += 2;
                } else if unicode >= 0x0800 {
                    let part1 = (0b11100000 | (0b00011111 & (unicode >> 12))) as u8;
                    let part2 = (0b10000000 | (0b00111111) & (unicode >> 6)) as u8;
                    let part3 = (0b10000000 | (0b00111111) & unicode) as u8;

                    utf8[len] = part1;
                    utf8[len + 1] = part2;
                    utf8[len + 2] = part3;
                    len += 3;
                }
            }
        };

        op(&self.unicode_part1);
        op(&self.unicode_part2);
        op(&self.unicode_part3);

        (utf8, len)
    }

    fn count_of_name(&self) -> usize {
        self.attribute as usize & 0x1F
    }

    fn is_name_end(&self) -> bool {
        (self.attribute & 0x40) == 0x40
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct DirectoryItem {
    pub(crate) item_type: ItemType,
    short: Option<ShortDirectoryItem>,
    long: Option<LongDirectoryItem>,
}

impl DirectoryItem {
    pub(crate) fn cluster(&self) -> u32 {
        self.short.unwrap().cluster
    }

    fn get_sfn(&self) -> Option<([u8; 12], usize)> {
        if self.short.is_some() {
            Some(self.short.unwrap().get_full_name_bytes())
        } else {
            None
        }
    }

    fn get_lfn(&self) -> Option<([u8; 13 * 3], usize)> {
        if self.long.is_some() {
            Some(self.long.unwrap().to_utf8())
        } else {
            None
        }
    }

    pub(crate) fn count_of_name(&self) -> Option<usize> {
        if self.long.is_some() {
            Some(self.long.unwrap().count_of_name())
        } else {
            None
        }
    }

    pub(crate) fn is_name_end(&self) -> Option<bool> {
        if self.long.is_some() {
            Some(self.long.unwrap().is_name_end())
        } else {
            None
        }
    }

    pub(crate) fn length(&self) -> Option<usize> {
        if self.short.is_some() {
            Some(self.short.unwrap().length as usize)
        } else {
            None
        }
    }

    pub(crate) fn bytes(&self) -> [u8; 32] {
        if self.short.is_some() {
            self.short.unwrap().bytes(self.item_type)
        } else {
            [0; 32]
        }
    }

    pub(crate) fn new_sfn(cluster: u32, value: &str, create_type: CreateType) -> Self {
        Self {
            item_type: ItemType::from_create(create_type),
            short: Some(ShortDirectoryItem::new(cluster, value, create_type)),
            long: None
        }
    }

    pub(crate) fn root_dir(cluster: u32) -> Self {
        Self {
            short: Some(ShortDirectoryItem::root_dir(cluster)),
            ..Self::default()
        }
    }

    pub(crate) fn from_buf(buf: &[u8]) -> Self {
        let item_type = if buf[0x00] == 0xE5 {
            ItemType::Deleted
        } else {
            ItemType::from_value(buf[0x0B])
        };

        match item_type {
            ItemType::LFN => {
                Self {
                    item_type,
                    short: None,
                    long: Some(LongDirectoryItem::from_buf(buf))
                }
            }
            _ => {
                Self {
                    item_type,
                    short: Some(ShortDirectoryItem::from_buf(buf)),
                    long: None
                }
            }
        }
    }

    pub(crate) fn sfn_equal(&self, value: &str) -> bool {
        if self.is_deleted() { return false; }
        let option = self.get_sfn();
        if option.is_none() { return false; }
        let (bytes, len) = option.unwrap();
        if let Ok(res) = str::from_utf8(&bytes[0..len]) {
            value.eq_ignore_ascii_case(res)
        } else {
            false
        }
    }

    pub(crate) fn lfn_equal(&self, value: &str) -> bool {
        if self.is_deleted() { return false; }
        let option = self.get_lfn();
        if option.is_none() { return false; }
        let (bytes, len) = option.unwrap();
        if let Ok(res) = str::from_utf8(&bytes[0..len]) {
            value.eq_ignore_ascii_case(res)
        } else {
            false
        }
    }

    pub(crate) fn is_lfn(&self) -> bool {
        ItemType::LFN == self.item_type
    }

    pub(crate) fn is_deleted(&self) -> bool {
        ItemType::Deleted == self.item_type
    }

    pub(crate) fn is_dir(&self) -> bool {
        ItemType::Dir == self.item_type
    }

    pub(crate) fn is_file(&self) -> bool {
        ItemType::File == self.item_type
    }
}
