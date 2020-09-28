use core::convert::TryInto;
use core::str;

pub(crate) enum NameType {
    SFN,
    LFN,
}

pub(crate) fn is_fat32(value: &[u8]) -> bool {
    let file_system_str = str::from_utf8(&value[0..5]).unwrap();
    file_system_str.eq("FAT32")
}

pub(crate) fn read_le_u16(input: &[u8]) -> u16 {
    let (int_bytes, _) = input.split_at(core::mem::size_of::<u16>());
    u16::from_le_bytes(int_bytes.try_into().unwrap())
}

pub(crate) fn read_le_u32(input: &[u8]) -> u32 {
    let (int_bytes, _) = input.split_at(core::mem::size_of::<u32>());
    u32::from_le_bytes(int_bytes.try_into().unwrap())
}

pub(crate) fn is_illegal(chs: &str) -> bool {
    let illegal_char = "\\/:*?\"<>|";
    for ch in illegal_char.chars() {
        if chs.contains(ch) {
            return true;
        }
    }
    false
}

pub(crate) fn sfn_or_lfn(value: &str) -> NameType {
    let (name, extension) = match value.find('.') {
        Some(i) => (&value[0..i], &value[i + 1..]),
        None => (&value[0..], "")
    };

    if value.is_ascii()
        && value.contains(|ch: char| ch.is_ascii_lowercase())
        && !value.contains(' ')
        && !name.contains('.')
        && !extension.contains('.')
        && name.len() <= 8
        && extension.len() <= 3 {
        NameType::SFN
    } else {
        NameType::LFN
    }
}

pub(crate) fn get_count_of_lfn(value: usize) -> usize {
    if value % 13 == 0 { value / 13 } else { value / 13 + 1 }
}

pub(crate) fn get_lfn_index(value_str: &str, count: usize) -> usize {
    let num = 13 * (count - 1);
    value_str.chars().enumerate().nth(num).unwrap().0
}