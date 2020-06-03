use crate::bpb::BIOSParameterBlock;
use crate::dir::Dir;
use crate::BUFFER_SIZE;

/// BasicOperation trait
pub trait BasicOperation {
    type Error;
    fn read(&self, buf: &mut [u8], address: u32, number_of_blocks: u32) -> Result<(), Self::Error>;
    fn write(&self, buf: &[u8], address: u32, number_of_blocks: u32) -> Result<(), Self::Error>;
}

#[derive(Debug, Copy, Clone)]
pub struct Volume<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug
{
    base: BASE,
    bpb: BIOSParameterBlock,
}

impl<BASE> Volume<BASE>
    where BASE: BasicOperation + Clone + Copy,
          <BASE as BasicOperation>::Error: core::fmt::Debug {
    /// get volume
    pub fn new(base: BASE) -> Volume<BASE> {
        let mut buf = [0; BUFFER_SIZE];
        base.read(&mut buf, 0, 1).unwrap();

        let mut volume_label = [0; 11];
        for i in 0x47..0x52 {
            volume_label[i - 0x47] = buf[i];
        }

        let mut file_system = [0; 8];
        for i in 0x52..0x5A {
            file_system[i - 0x52] = buf[i];
        }

        let bps = ((buf[0x0C] as u16) << 8) | buf[0x0B] as u16;
        if bps as usize != BUFFER_SIZE {
            panic!("BUFFER_SIZE is {} Bytes, byte_per_sector is {} Bytes, no equal, \
            please edit feature {}", BUFFER_SIZE, bps, bps);
        }

        Volume::<BASE> {
            base,
            bpb: BIOSParameterBlock {
                byte_per_sector: bps,
                sector_per_cluster: buf[0x0D],
                reserved_sector: ((buf[0x0F] as u16) << 8) | buf[0x0E] as u16,
                fat_count: buf[0x10],
                all_sector: ((buf[0x23] as u32) << 24)
                    | ((buf[0x22] as u32) << 16)
                    | ((buf[0x21] as u32) << 8)
                    | (buf[0x20] as u32),
                sector_per_fat: ((buf[0x27] as u32) << 24)
                    | ((buf[0x26] as u32) << 16)
                    | ((buf[0x25] as u32) << 8)
                    | (buf[0x24] as u32),
                root_cluster: ((buf[0x2F] as u32) << 24)
                    | ((buf[0x2E] as u32) << 16)
                    | ((buf[0x2D] as u32) << 8)
                    | (buf[0x2C] as u32),
                id: ((buf[0x46] as u32) << 24)
                    | ((buf[0x45] as u32) << 16)
                    | ((buf[0x44] as u32) << 8)
                    | (buf[0x43] as u32),
                volume_label,
                file_system,
            },
        }
    }

    /// into root_dir
    pub fn root_dir(&self) -> Dir<BASE> {
        Dir::<BASE> {
            base: self.base,
            bpb: self.bpb,
            dir_name: [0; 11],
            create_ms: 0,
            create_time: [0; 2],
            create_date: [0; 2],
            visit_date: [0; 2],
            edit_time: [0; 2],
            edit_date: [0; 2],
            dir_cluster: self.bpb.root_cluster,
            length: 0,
        }
    }
}