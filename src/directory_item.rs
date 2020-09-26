use crate::tool::read_le_u32;

#[derive(Copy, Clone, Debug)]
pub enum ItemType {
    Dir,
    File,
}

impl ItemType {
    fn from_value(value: u8) -> ItemType {
        if (value & 0x10) == 0x10 {
            ItemType::Dir
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
pub struct DirectoryItem {
    pub(crate) item_type: ItemType,
    pub(crate) name: [u8; 8],
    pub(crate) extension: [u8; 3],
    pub(crate) create_ms: u8,
    pub(crate) create_time: [u8; 2],
    pub(crate) create_date: [u8; 2],
    pub(crate) last_visit_date: [u8; 2],
    pub(crate) edit_time: [u8; 2],
    pub(crate) edit_date: [u8; 2],
    pub(crate) length: u32,
    pub(crate) cluster: u32,
}


impl DirectoryItem {
    pub fn root_dir(cluster: u32) -> Self {
        Self {
            cluster,
            ..Self::default()
        }
    }

    pub fn from_buf(buf: &[u8]) -> Self {
        let mut name = [0; 8];
        let mut extension = [0; 3];
        let create_time = [buf[0x0F], buf[0x0E]];
        let create_date = [buf[0x11], buf[0x10]];
        let last_visit_date = [buf[0x13], buf[0x12]];
        let edit_time = [buf[0x17], buf[0x16]];
        let edit_date = [buf[0x19], buf[0x18]];
        let item_type = ItemType::from_value(buf[0x0B]);

        for i in 0x00..0x08 { name[i] = buf[i]; }
        for i in 0x08..0x0B { extension[i] = buf[i]; }

        Self {
            item_type,
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
}