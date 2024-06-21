use stm32f4xx_hal::otg_fs::{UsbBus, USB};
use usbd_hid::{
    //descriptor::MouseReport,
    hid_class::HIDClass,
};

use usbd_hid::descriptor::generator_prelude::Serialize;
use usbd_hid::descriptor::generator_prelude::SerializeTuple;
use usbd_hid::descriptor::generator_prelude::Serializer;
use usbd_hid::descriptor::AsInputReport;
use usbd_hid::descriptor::SerializedDescriptor;
use usbd_hid_macros::gen_hid_descriptor;

use crate::config::*;
use crate::mode::DeviceMode;
use crate::remote::RcButton;

#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = MOUSE) = {
        (collection = PHYSICAL, usage = POINTER) = {
            #[item_settings constant,variable,absolute] report_id=input;
            (usage_page = BUTTON, usage_min = BUTTON_1, usage_max = BUTTON_8) = {
                #[packed_bits 8] #[item_settings data,variable,absolute] buttons=input;
            };
            (usage_page = GENERIC_DESKTOP,) = {
                (usage = X,) = {
                    #[item_settings data,variable,relative] x=input;
                };
                (usage = Y,) = {
                    #[item_settings data,variable,relative] y=input;
                };
                (usage = WHEEL,) = {
                    #[item_settings data,variable,relative] wheel=input;
                };
            };
            (usage_page = CONSUMER,) = {
                (usage = AC_PAN,) = {
                    #[item_settings data,variable,relative] pan=input;
                };
            };
        };
    }
)]
#[allow(dead_code)]
pub struct MouseReportEx {
    pub report_id: u8,
    pub buttons: u8,
    pub x: i8,
    pub y: i8,
    pub wheel: i8, // Scroll down (negative) or up (positive) this many units
    pub pan: i8,   // Scroll left (negative) or right (positive) this many units
}

pub fn handle_mouse_event(
    hid: &mut HIDClass<'static, UsbBus<USB>>,
    button: RcButton,
    speed: u8,
) -> DeviceMode {
    let mut pointer_x = 0;
    let mut pointer_y = 0;
    let mut wheel = 0;
    let mut pan = 0;
    let mut buttons = 0;
    let mut release = false;
    let mut double = false;

    const REPORT_ID: u8 = 1;
    const MOVE_STEPS: [i8; 4] = [10, 25, 60, 127];
    let move_step = MOVE_STEPS[usize::from(speed)];

    match button {
        RcButton::Up => {
            pointer_y = -move_step;
        }
        RcButton::Down => {
            pointer_y = move_step;
        }
        RcButton::Left => {
            pointer_x = -move_step;
        }
        RcButton::Right => {
            pointer_x = move_step;
        }
        RcButton::VolumeUp => {
            wheel = 1;
        }
        RcButton::VolumeDown => {
            wheel = -1;
        }
        RcButton::PageUp => {
            pan = 1;
        }
        RcButton::PageDown => {
            pan = -1;
        }
        RcButton::Text => {
            pointer_y = -move_step;
            pointer_x = -move_step;
        }
        RcButton::MyApps => {
            pointer_y = -move_step;
            pointer_x = move_step;
        }
        RcButton::Back => {
            pointer_y = move_step;
            pointer_x = -move_step;
        }
        RcButton::Exit => {
            pointer_y = move_step;
            pointer_x = move_step;
        }
        RcButton::Ok => {
            buttons = 0b00000001;
            release = true;
        }
        RcButton::Netflix => {
            buttons = 0b00000001;
            release = true;
            double = true;
        }
        RcButton::Start => {
            buttons = 0b00000100;
            release = true;
        }
        RcButton::Amazon => {
            buttons = 0b00000010;
            release = true;
        }
        //RcButton::Red        => {return DeviceMode::Mouse},
        RcButton::Green => return DeviceMode::Keyboard,
        _ => return DeviceMode::Mouse,
    }

    loop {
        let report = MouseReportEx {
            report_id: REPORT_ID,
            x: pointer_x,
            y: pointer_y,
            buttons: buttons,
            wheel: wheel,
            pan: pan,
        };

        hid.push_input(&report).ok();

        if release {
            cortex_m::asm::delay(MOUSE_DOUBLE_CLICK_DELAY);

            let report = MouseReportEx {
                report_id: REPORT_ID,
                x: 0,
                y: 0,
                buttons: 0,
                wheel: 0,
                pan: 0,
            };
            hid.push_input(&report).ok();
        }

        if double {
            cortex_m::asm::delay(MOUSE_BUTTON_RELEASE_DELAY);
            double = false;
            continue;
        }

        break;
    }

    return DeviceMode::Mouse;
}
