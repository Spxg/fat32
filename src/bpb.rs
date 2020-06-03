#[derive(Debug, Copy, Clone)]
pub struct BIOSParameterBlock {
    pub(crate) byte_per_sector: u16,
    pub(crate) sector_per_cluster: u8,
    pub(crate) reserved_sector: u16,
    pub(crate) fat_count: u8,
    pub(crate) all_sector: u32,
    pub(crate) sector_per_fat: u32,
    pub(crate) root_cluster: u32,
    pub(crate) id: u32,
    pub(crate) volume_label: [u8; 11],
    pub(crate) file_system: [u8; 8],
}

impl BIOSParameterBlock {
    pub(crate) fn offset(&self, cluster: u32) -> u32 {
        ((self.reserved_sector as u32)
            + (self.fat_count as u32) * self.sector_per_fat
            + (cluster - 2) * (self.sector_per_cluster as u32))
            * (self.byte_per_sector as u32)
    }

    pub(crate) fn fat1(&self) -> u32 {
        (self.reserved_sector as u32) * (self.byte_per_sector as u32)
    }
}