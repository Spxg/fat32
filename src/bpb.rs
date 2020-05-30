#[derive(Debug, Copy, Clone)]
pub struct BIOSParameterBlock {
    pub byte_per_sector: u16,
    pub sector_per_cluster: u8,
    pub reserved_sector: u16,
    pub fat_count: u8,
    pub all_sector: u32,
    pub sector_per_fat: u32,
    pub root_cluster: u32,
    pub id: u32,
    pub volume_label: [u8; 11],
    pub file_system: [u8; 8],
}

impl BIOSParameterBlock {
    pub fn offset(&self, cluster: u32) -> u32 {
        ((self.reserved_sector as u32)
            + (self.fat_count as u32) * self.sector_per_fat
            + (cluster - 2) * (self.sector_per_cluster as u32))
            * (self.byte_per_sector as u32)
    }

    pub fn fat1(&self) -> u32 {
        (self.reserved_sector as u32) * (self.byte_per_sector as u32)
    }
}