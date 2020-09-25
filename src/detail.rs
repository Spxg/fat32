#[derive(Default, Copy, Clone, Debug)]
pub struct Detail {
    pub(crate) dir_name: [u8; 11],
    pub(crate) create_ms: u8,
    pub(crate) create_time: [u8; 2],
    pub(crate) create_date: [u8; 2],
    pub(crate) last_visit_date: [u8; 2],
    pub(crate) edit_time: [u8; 2],
    pub(crate) edit_date: [u8; 2],
    pub(crate) length: u32,
}

