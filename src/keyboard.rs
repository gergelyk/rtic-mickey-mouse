use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use usbd_hid::{
    //descriptor::KeyboardReport,
    //descriptor::MediaKeyboardReport,
    descriptor::MediaKey,
    hid_class::HIDClass,
};

use usbd_hid::descriptor::generator_prelude::Serialize;
use usbd_hid::descriptor::generator_prelude::SerializeTuple;
use usbd_hid::descriptor::generator_prelude::Serializer;
use usbd_hid::descriptor::AsInputReport;
use usbd_hid::descriptor::KeyboardUsage;
use usbd_hid::descriptor::SerializedDescriptor;
use usbd_hid_macros::gen_hid_descriptor;

use crate::config::*;
use crate::mode::DeviceMode;
use crate::remote::RcButton;

#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = KEYBOARD) = {
        #[item_settings constant,variable,absolute] report_id=input;
        (usage_page = KEYBOARD, usage_min = 0xE0, usage_max = 0xE7) = {
            #[packed_bits 8] #[item_settings data,variable,absolute] modifier=input;
        };
        (usage_min = 0x00, usage_max = 0xFF) = {
            #[item_settings constant,variable,absolute] reserved=input;
        };
        (usage_page = LEDS, usage_min = 0x01, usage_max = 0x05) = {
            #[packed_bits 5] #[item_settings data,variable,absolute] leds=output;
        };
        (usage_page = KEYBOARD, usage_min = 0x00, usage_max = 0xDD) = {
            #[item_settings data,array,absolute] keycodes=input;
        };
    }
)]
#[allow(dead_code)]
pub struct KeyboardReportEx {
    pub report_id: u8,
    pub modifier: u8,
    pub reserved: u8,
    pub leds: u8,
    pub keycodes: [u8; 6],
}

#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = CONSUMER, usage = CONSUMER_CONTROL) = {
        #[item_settings constant,variable,absolute] report_id=input;
        (usage_page = CONSUMER, usage_min = 0x00, usage_max = 0x514) = {
            #[item_settings data,array,absolute,not_null] usage_id=input;
        };
    }
)]
#[allow(dead_code)]
pub struct MediaKeyboardReportEx {
    pub report_id: u8,
    pub usage_id: u16,
}

#[derive(Clone)]
enum GenericKeyboardKey {
    KeyboardKey(KeyboardUsage),
    MediaKey(MediaKey),
}

fn send_key(hid: &mut HIDClass<'static, UsbBus<USB>>, key: &GenericKeyboardKey) {
    const KEYBOARD_REPORT_ID: u8 = 2;
    const MEDIA_KEYBOARD_REPORT_ID: u8 = 3;

    match key {
        GenericKeyboardKey::KeyboardKey(key) => {
            let report = KeyboardReportEx {
                report_id: KEYBOARD_REPORT_ID,
                modifier: 0,
                leds: 0,
                reserved: 0,
                keycodes: [*key as u8, 0, 0, 0, 0, 0],
            };
            hid.push_input(&report).ok();
        }
        GenericKeyboardKey::MediaKey(key) => {
            let report = MediaKeyboardReportEx {
                report_id: MEDIA_KEYBOARD_REPORT_ID,
                usage_id: *key as u16,
            };
            hid.push_input(&report).ok();
        }
    }
}

fn release_key(hid: &mut HIDClass<'static, UsbBus<USB>>, key: &GenericKeyboardKey) {
    const KEYBOARD_REPORT_ID: u8 = 2;
    const MEDIA_KEYBOARD_REPORT_ID: u8 = 3;

    match key {
        GenericKeyboardKey::KeyboardKey(_) => {
            let report = KeyboardReportEx {
                report_id: KEYBOARD_REPORT_ID,
                modifier: 0,
                leds: 0,
                reserved: 0,
                keycodes: [0, 0, 0, 0, 0, 0],
            };
            hid.push_input(&report).ok();
        }
        GenericKeyboardKey::MediaKey(_) => {
            let report = MediaKeyboardReportEx {
                report_id: MEDIA_KEYBOARD_REPORT_ID,
                usage_id: 0,
            };
            hid.push_input(&report).ok();
        }
    }
}

pub fn handle_keyboard_event(
    hid: &mut HIDClass<'static, UsbBus<USB>>,
    button: RcButton,
    _speed: u8,
) -> DeviceMode {
    let key: GenericKeyboardKey;

    match button {
        RcButton::Up => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardUpArrow);
        }
        RcButton::Down => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardDownArrow);
        }
        RcButton::Left => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardLeftArrow);
        }
        RcButton::Right => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardRightArrow);
        }
        RcButton::Ok => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardEnter);
        }
        RcButton::Text => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardHome);
        }
        RcButton::MyApps => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardEnd);
        }
        RcButton::Exit => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardEscape);
        }
        RcButton::PageUp => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardPageUp);
        }
        RcButton::PageDown => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardPageDown);
        }
        RcButton::VolumeUp => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardVolumeUp);
        }
        RcButton::VolumeDown => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardVolumeDown);
        }
        RcButton::Mute => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardMute);
        }
        RcButton::Netflix => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardBackspace);
        }
        RcButton::Start => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardSpacebar);
        }
        RcButton::Amazon => {
            key = GenericKeyboardKey::KeyboardKey(KeyboardUsage::KeyboardDelete);
        }
        RcButton::Record => {
            key = GenericKeyboardKey::MediaKey(MediaKey::Record);
        }
        RcButton::Stop => {
            key = GenericKeyboardKey::MediaKey(MediaKey::Stop);
        }
        RcButton::Play => {
            key = GenericKeyboardKey::MediaKey(MediaKey::Play);
        }
        RcButton::Pause => {
            key = GenericKeyboardKey::MediaKey(MediaKey::Pause);
        }
        RcButton::NextTrack => {
            key = GenericKeyboardKey::MediaKey(MediaKey::NextTrack);
        }
        RcButton::PrevTrack => {
            key = GenericKeyboardKey::MediaKey(MediaKey::PrevTrack);
        }
        RcButton::Red => return DeviceMode::Mouse,
        //RcButton::Green      => {return DeviceMode::Keyboard},
        _ => return DeviceMode::Keyboard,
    }

    send_key(hid, &key);
    cortex_m::asm::delay(KEYBOARD_BUTTON_RELEASE_DELAY);
    release_key(hid, &key);

    return DeviceMode::Keyboard;
}
