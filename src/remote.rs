use int_enum::IntEnum;

// keycode format: FF0UUUUULLLLLLLL
// each letter is a nibble where:
// F - flags, U - upper code, L - lower code, 0 - not used
const NIBBLE_WIDTH: u32 = 4;
pub const FLAGS_OFFSET: u32 = 14 * NIBBLE_WIDTH;
const UPPER_WIDTH: u32 = 5 * NIBBLE_WIDTH;
const LOWER_WIDTH: u32 = 8 * NIBBLE_WIDTH;
pub const DATA_WIDTH: u32 = UPPER_WIDTH + LOWER_WIDTH;

pub fn decode_keycode(keycode: u64) -> (u32, u32, bool) {
    let flags: u8 = (keycode >> FLAGS_OFFSET).try_into().unwrap();
    let flag_repeated = 0x1 & flags != 0;
    let upper_code: u32 = (0x000fffff & (keycode >> 8 * NIBBLE_WIDTH))
        .try_into()
        .unwrap();
    let lower_code: u32 = (keycode & 0xffffffff).try_into().unwrap();
    return (upper_code, lower_code, flag_repeated);
}

#[repr(u32)]
#[derive(IntEnum)]
pub enum RcButton {
    Up = 0x5012aa97,
    Down = 0x5408aa97,
    Left = 0x55401557,
    Right = 0x52811557,
    Ok = 0x51094a97,
    Text = 0x5022aa57,
    MyApps = 0x52092a97,
    Back = 0x50915257,
    Exit = 0x55290897,
    PageUp = 0x50055557,
    PageDown = 0x54015557,
    VolumeUp = 0x52025557,
    VolumeDown = 0x55005557,
    Mute = 0x5440a557,
    Netflix = 0x52924497,
    Start = 0x51552817,
    Amazon = 0x51525097,
    Red = 0x522a4a17,
    Green = 0x542a2a17,
    Record = 0x54aa4827,
    Stop = 0x54292a27,
    PrevTrack = 0x555082a7,
    Play = 0x5052aa27,
    Pause = 0x5254a427,
    NextTrack = 0x52a142a7,
    // more buttons to come as they are needed
}
