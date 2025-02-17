use core::fmt::Write;
use embassy_net::new;
use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal_async::i2c::I2c as AsyncI2c;
use heapless::{String, Vec};
use ssd1306::prelude::DisplaySize128x64;
use ssd1306::{mode::BufferedGraphicsModeAsync, prelude::*, Ssd1306Async};

// USE SAME FONT FOR SIMPLIFICATION
const DISPLAY_FONT: embedded_graphics::mono_font::MonoFont =
    embedded_graphics::mono_font::ascii::FONT_5X8;
const DISPLAY_WIDTH: usize = 128;

pub enum MqttCode {
    Offline,
    Connected,
    Disconnected,
    Published,
    Err,
}

impl MqttCode {
    pub fn to_str(&self) -> &'static str {
        match self {
            MqttCode::Offline => "Offline",
            MqttCode::Connected => "Connected",
            MqttCode::Disconnected => "Disconnected",
            MqttCode::Published => "Published",
            MqttCode::Err => "Err",
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => MqttCode::Offline,
            1 => MqttCode::Connected,
            2 => MqttCode::Disconnected,
            3 => MqttCode::Published,
            _ => MqttCode::Err,
        }
    }
}

pub struct MsgLevelUnit {
    pub msg: &'static str,
    pub level: u8,
    pub unit: &'static str,
}

impl MsgLevelUnit {
    pub fn new(msg: &'static str, level: u8, unit: &'static str) -> MsgLevelUnit {
        MsgLevelUnit { msg, level, unit }
    }

    pub fn msg(&self) -> &'static str {
        self.msg
    }

    pub fn level(&self) -> u8 {
        self.level
    }
    pub fn set_level(&mut self, level: u8) {
        self.level = level;
    }

    pub fn unit(&self) -> &'static str {
        self.unit
    }
    // Max of 16 + 4 + 4 chars for level and unit
    pub fn to_string(&self) -> String<8> {
        let mut s = String::<8>::new();
        write!(
            &mut s,
            "{:15} {:3} {:4}",
            if self.msg.len() <= 15 {
                self.msg
            } else {
                &self.msg[..15]
            },
            self.level,
            if self.unit.len() <= 4 {
                self.unit
            } else {
                &self.unit[..4]
            }
        )
        .unwrap();
        s
    }
}

pub struct DisplayData {
    pub wifi: MsgLevelUnit,
    pub mqtt_client: MsgLevelUnit,
    pub device_time: &'static str,
}

pub fn configure_text_style() -> MonoTextStyle<'static, BinaryColor> {
    MonoTextStyleBuilder::new()
        .font(&DISPLAY_FONT)
        .text_color(BinaryColor::On)
        .build()
}

pub async fn display_message<D>(
    display: &mut Ssd1306Async<
        I2CInterface<D>,
        DisplaySize128x64,
        BufferedGraphicsModeAsync<DisplaySize128x64>,
    >,
    text_style: &MonoTextStyle<'_, BinaryColor>,
    data: DisplayData,
) where
    D: AsyncI2c,
{
    // Display time first
    Text::with_baseline(data.device_time, Point::zero(), *text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    // Skip lines and display other data
    let font_height = &DISPLAY_FONT.character_size.height;
    let mut y: i32 = (*font_height).try_into().unwrap();

    let mut message = data.wifi.to_string();
    // HACK: WILL NOT WORK FOR DIFF FONTS
    Text::with_baseline(&message, Point::new(0, y), *text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    message = data.mqtt_client.to_string();
    y = (*font_height * 2).try_into().unwrap();
    Text::with_baseline(&message, Point::new(0, y), *text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    display.flush().await.unwrap();
}

//pub async fn clear_line<D>(
//    display: &mut Ssd1306Async<
//        I2CInterface<D>,
//        DisplaySize128x64,
//        BufferedGraphicsModeAsync<DisplaySize128x64>,
//    >,
//    y: u8, // y-coordinate of the line to clear
//) where
//    D: AsyncI2c,
//{
//    // HACK: heapless does not allow dynamic size when alloc
//    let buffer = Vec::<u8, DISPLAY_WIDTH>::new();
//
//    let upper_left = (0, y); // Starting at (0, y)
//    let lower_right = (DISPLAY_WIDTH as u8 - 1, y); // Full width, same y-coordinate
//
//    display
//        .bounded_draw(&buffer, DISPLAY_WIDTH, upper_left, lower_right)
//        .await
//        .unwrap();
//
//    display.flush().await.unwrap(); // Ensure the changes are sent to the display
//}
