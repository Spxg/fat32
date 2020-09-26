#[derive(Default, Copy, Clone, Debug)]
pub struct DirectoryItem {
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
}