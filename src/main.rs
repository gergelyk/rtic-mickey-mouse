#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use rtic_mickey_mouse as _;

mod config;
mod descriptor;
mod keyboard;
mod mode;
mod mouse;
mod remote;

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [SPI1])]
mod app {

    use core::mem::MaybeUninit;
    use fugit::ExtU32;
    use rtic_monotonics::{rtic_time::embedded_hal_async::delay::DelayNs, stm32::prelude::*};
    use rtic_sync::{channel::*, make_channel};
    use stm32f4xx_hal::gpio::{gpioa::PA0, gpioa::PA1, gpiob::PB9, gpioc::PC13};
    use stm32f4xx_hal::gpio::{Edge, ExtiPin, Input, Output, PushPull};
    use stm32f4xx_hal::otg_fs::{UsbBus, UsbBusType, USB};
    use stm32f4xx_hal::prelude::*;
    use usb_device::{bus::UsbBusAllocator, prelude::*};
    use usbd_hid::hid_class::HIDClass;

    use crate::config::*;
    use crate::descriptor::HID_DESCRIPTOR;
    use crate::keyboard;
    use crate::mode::DeviceMode;
    use crate::mouse;
    use crate::remote;
    use crate::remote::{decode_keycode, RcButton};

    stm32_tim2_monotonic!(Mono, 25_000_000); // tick rate [Hz]

    #[shared]
    struct Shared {
        hid: HIDClass<'static, UsbBusType>,
        btn: PA0<Input>,
        ir: PB9<Input>,
        led: PC13<Output<PushPull>>,
        enabled: bool,
    }

    #[local]
    struct Local {
        usb_dev: UsbDevice<'static, UsbBus<USB>>,
        sample_clk: PA1<Output<PushPull>>,
        keycode_tx: Sender<'static, u64, 10>,
    }

    #[init(local = [ep_memory: [u32; 1024] = [0; 1024], usb_bus: MaybeUninit<UsbBusAllocator<UsbBusType>> = MaybeUninit::uninit()])]
    fn init(mut ctx: init::Context) -> (Shared, Local) {
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.MHz()).require_pll48clk().freeze();

        Mono::start(25_000_000); // timer clock [Hz]

        let gpioa = ctx.device.GPIOA.split();
        let gpiob = ctx.device.GPIOB.split();
        let gpioc = ctx.device.GPIOC.split();

        let mut sys_cfg = ctx.device.SYSCFG.constrain();

        let mut btn = gpioa.pa0.into_pull_up_input();
        btn.make_interrupt_source(&mut sys_cfg);
        btn.enable_interrupt(&mut ctx.device.EXTI);
        btn.trigger_on_edge(&mut ctx.device.EXTI, Edge::RisingFalling);

        let mut ir = gpiob.pb9.into_pull_up_input();
        ir.make_interrupt_source(&mut sys_cfg);
        ir.enable_interrupt(&mut ctx.device.EXTI);
        ir.trigger_on_edge(&mut ctx.device.EXTI, Edge::RisingFalling);

        let mut led = gpioc.pc13.into_push_pull_output();
        led.set_low();

        let mut sample_clk = gpioa.pa1.into_push_pull_output();
        sample_clk.set_high();

        let usb = USB {
            usb_global: ctx.device.OTG_FS_GLOBAL,
            usb_device: ctx.device.OTG_FS_DEVICE,
            usb_pwrclk: ctx.device.OTG_FS_PWRCLK,
            pin_dm: gpioa.pa11.into(),
            pin_dp: gpioa.pa12.into(),
            hclk: clocks.hclk(),
        };

        let usb_bus = ctx.local.usb_bus;
        let usb_bus = usb_bus.write(UsbBus::new(usb, ctx.local.ep_memory));

        let hid = HIDClass::new(usb_bus, HID_DESCRIPTOR, 60);

        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x05df, 0x16c0))
            .strings(&[StringDescriptors::default()
                .manufacturer("krason.dev")
                .product("MicKeyMouse")
                .serial_number("0001")])
            .unwrap()
            .device_class(0)
            .build();

        let (keycode_tx, keycode_rx) = make_channel!(u64, 10);
        let enabled = true;

        receiver_task::spawn(keycode_rx).unwrap();

        (
            Shared {
                hid,
                btn,
                ir,
                led,
                enabled,
            },
            Local {
                usb_dev,
                sample_clk,
                keycode_tx,
            },
        )
    }

    #[task(shared = [hid])]
    async fn receiver_task(
        ctx: receiver_task::Context,
        mut keycode_rx: Receiver<'static, u64, 10>,
    ) {
        let mut hid = ctx.shared.hid;
        const MAX_SPEED: u8 = 3;
        let mut speed: u8 = 0;
        let mut device_mode: DeviceMode = DeviceMode::Mouse;

        loop {
            while let Ok(keycode) = keycode_rx.recv().await {
                let (upper_code, lower_code, flag_repeated) = decode_keycode(keycode);
                if upper_code != MAGIC_PREFIX {
                    continue;
                }

                if flag_repeated {
                    if speed < MAX_SPEED {
                        speed += 1;
                    }
                } else {
                    speed = 0;
                }

                defmt::println!(
                    "lower_code={:#010x}, repeated={}, speed={}",
                    lower_code,
                    flag_repeated,
                    speed
                );

                let maybe_button = RcButton::try_from(lower_code);
                match maybe_button {
                    Ok(button) => {
                        hid.lock(|hid| {
                            device_mode = match device_mode {
                                DeviceMode::Mouse => mouse::handle_mouse_event(hid, button, speed),
                                DeviceMode::Keyboard => {
                                    keyboard::handle_keyboard_event(hid, button, speed)
                                }
                            };
                        });
                    }
                    Err(_) => {}
                }
            }
        }
    }

    #[task(priority=1, local = [keycode_tx, sample_clk, last_ticks : u64 = 0, last_keycode : u64 = 0], shared = [ir, led])]
    async fn sample_clk_task(ctx: sample_clk_task::Context) {
        let timestamp = Mono::now();
        let sample_clk = ctx.local.sample_clk;
        let last_keycode = ctx.local.last_keycode;
        let last_ticks = ctx.local.last_ticks;
        let keycode_tx = ctx.local.keycode_tx;
        let mut led = ctx.shared.led;
        let mut ir = ctx.shared.ir;
        let mut keycode: u64 = 0;

        let interval = SAMPLE_INTERVAL_US.micros();
        let offset = SAMPLE_OFFSET_US.micros();

        for sample_cnt in 0u32..(remote::DATA_WIDTH) {
            Mono::delay_until(timestamp + interval * sample_cnt + offset).await;
            sample_clk.toggle();
            keycode <<= 1;
            if ir.lock(|pin| pin.is_high()) {
                keycode |= 1;
            }
        }

        let ticks = timestamp.ticks();
        let delta = ticks.wrapping_sub(*last_ticks);
        let mut flags: u8 = 0;
        //defmt::println!("delta= {}", delta);
        if *last_keycode == keycode {
            if delta < MAX_REPETITION_INTERVAL {
                flags |= 0x1;
            }
        }
        *last_ticks = ticks;
        *last_keycode = keycode;

        keycode = (u64::from(flags) << remote::FLAGS_OFFSET) | keycode;

        sample_clk.set_high();
        //defmt::println!("keycode={:#018x}", keycode);
        let _ = keycode_tx.send(keycode).await;
        led.lock(|pin| pin.set_high());
        DelayNs::delay_ms(&mut Mono, BLINK_DURATION_MS).await;
        led.lock(|pin| pin.set_low());
    }

    #[task(binds = EXTI9_5, local = [last_ticks : u64 = 0], shared = [ir, enabled])]
    fn on_ir(mut ctx: on_ir::Context) {
        ctx.shared.ir.lock(ExtiPin::clear_interrupt_pending_bit);
        let last_ticks = ctx.local.last_ticks;
        let mut enabled = ctx.shared.enabled;

        let timestamp = Mono::now();
        let ticks = timestamp.ticks();
        let delta = ticks.wrapping_sub(*last_ticks);
        *last_ticks = ticks;

        if PREAMBLE_REFERENCE - PREAMBLE_TOLERANCE <= delta
            && delta < PREAMBLE_REFERENCE + PREAMBLE_TOLERANCE
        {
            enabled.lock(|enabled| {
                if *enabled {
                    sample_clk_task::spawn().ok();
                }
            });
        }
    }

    #[task(binds=OTG_FS, local = [usb_dev], shared = [hid])]
    fn on_usb(ctx: on_usb::Context) {
        let usb_dev = ctx.local.usb_dev;
        let mut hid = ctx.shared.hid;
        hid.lock(|hid| if !usb_dev.poll(&mut [hid]) {});
    }

    #[task(binds = EXTI0, shared = [btn, led, enabled])]
    fn on_btn(ctx: on_btn::Context) {
        let mut btn = ctx.shared.btn;
        let mut led = ctx.shared.led;
        let mut enabled = ctx.shared.enabled;

        btn.lock(ExtiPin::clear_interrupt_pending_bit);
        enabled.lock(|enabled| {
            btn.lock(|btn| {
                if btn.is_low() {
                    if *enabled {
                        defmt::println!("disabled");
                        *enabled = false;
                        led.lock(|pin| pin.set_high());
                    } else {
                        defmt::println!("enabled");
                        *enabled = true;
                        led.lock(|pin| pin.set_low());
                    }
                }
            });
        });
        cortex_m::asm::delay(DEBOUNCE_DELAY);
    }
}
