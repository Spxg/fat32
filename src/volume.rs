use block_device::BlockDevice;
use core::convert::TryInto;
use core::str;
use crate::bpb::BIOSParameterBlock;
use crate::BUFFER_SIZE;

#[derive(Debug, Copy, Clone)]
pub struct Volume<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug
{
    device: T,
    bpb: BIOSParameterBlock,
}

impl<T> Volume<T>
    where T: BlockDevice + Clone + Copy,
          <T as BlockDevice>::Error: core::fmt::Debug {
    /// get volume
    pub fn new(device: T) -> Volume<T> {
        let mut buf = [0; BUFFER_SIZE];
        device.read(&mut buf, 0, 1).unwrap();

        let mut volume_label = [0; 11];
        volume_label.copy_from_slice(&buf[0x47..0x52]);

        let mut file_system = [0; 8];
        file_system.copy_from_slice(&buf[0x52..0x5A]);

        if !is_fat32(&file_system) { panic!("not fat32 file_system"); }

        let bps = read_le_u16(&buf[0x0B..0x0D]);
        if bps as usize != BUFFER_SIZE {
            panic!("BUFFER_SIZE is {} Bytes, byte_per_sector is {} Bytes, no equal, \
            please edit feature {}", BUFFER_SIZE, bps, bps);
        }

        Volume::<T> {
            device,
            bpb: BIOSParameterBlock {
                byte_per_sector: bps,
                sector_per_cluster: buf[0x0D],
                reserved_sector: read_le_u16(&buf[0x0D..0x0F]),
                num_fat: buf[0x10],
                total_sector: read_le_u32(&buf[0x20..0x24]),
                sector_per_fat: read_le_u32(&buf[0x24..0x28]),
                root_cluster: read_le_u32(&buf[0x2C..0x30]),
                id: read_le_u32(&buf[0x43..0x47]),
                volume_label,
                file_system,
            },
        }
    }
}

fn is_fat32(value: &[u8]) -> bool {
    let file_system_str = str::from_utf8(&value[0..5]).unwrap();
    file_system_str.eq("FAT32")
}

pub fn read_le_u16(input: &[u8]) -> u16 {
    let (int_bytes, _) = input.split_at(core::mem::size_of::<u16>());
    u16::from_le_bytes(int_bytes.try_into().unwrap())
}

pub fn read_le_u32(input: &[u8]) -> u32 {
    let (int_bytes, _) = input.split_at(core::mem::size_of::<u32>());
    u32::from_le_bytes(int_bytes.try_into().unwrap())
}