use core::fmt::Write;
use embedded_graphics::{
    mono_font::{ascii, MonoFont, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal_async::i2c::I2c as AsyncI2c;
use esp_hal::time::Duration;
use heapless::String;
use ssd1306::prelude::DisplaySize128x64;
use ssd1306::{mode::BufferedGraphicsModeAsync, prelude::*, Ssd1306Async};

// USE SAME FONT FOR SIMPLIFICATION
const DISPLAY_FONT: MonoFont = ascii::FONT_5X8;
//const DISPLAY_WIDTH: usize = 128;

pub trait LevelUnit {
    fn msg(&self) -> &'static str;
    fn level(&self) -> u8;
    fn unit(&self) -> &'static str;

    // Max of 16 + 4 + 4 chars for level and unit
    fn to_string(&self) -> String<24> {
        let mut s = String::<24>::new();
        write!(
            &mut s,
            "{:15} {:3} {:4}",
            if self.msg().len() <= 15 {
                self.msg()
            } else {
                &self.msg()[..15]
            },
            self.level(),
            if self.unit().len() <= 4 {
                self.unit()
            } else {
                &self.unit()[..4]
            }
        )
        .unwrap();
        s
    }
}

pub struct WifiLevelUnit {
    pub msg: &'static str,
    pub level: u8,
    pub unit: &'static str,
}

impl WifiLevelUnit {
    pub fn new(msg: &'static str, level: u8, unit: &'static str) -> WifiLevelUnit {
        WifiLevelUnit { msg, level, unit }
    }
    pub fn set_level(&mut self, level: u8) {
        self.level = level;
    }
}

impl LevelUnit for WifiLevelUnit {
    fn msg(&self) -> &'static str {
        self.msg
    }
    fn level(&self) -> u8 {
        self.level
    }
    fn unit(&self) -> &'static str {
        self.unit
    }
}

pub enum MqttStatus {
    Offline,
    Connected,
    Disconnected,
    Published,
    Err,
}

impl MqttStatus {
    pub fn to_str(&self) -> &'static str {
        match self {
            MqttStatus::Offline => "Offline",
            MqttStatus::Connected => "Connected",
            MqttStatus::Disconnected => "Disconnected",
            MqttStatus::Published => "Published",
            MqttStatus::Err => "Error",
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => MqttStatus::Offline,
            1 => MqttStatus::Connected,
            2 => MqttStatus::Disconnected,
            3 => MqttStatus::Published,
            _ => MqttStatus::Err,
        }
    }
}

pub struct MqttLevelUnit {
    pub msg: &'static str,
    pub level: u8,
    pub unit: MqttStatus,
}

impl MqttLevelUnit {
    pub fn new(msg: &'static str, level: u8) -> MqttLevelUnit {
        MqttLevelUnit {
            msg,
            level,
            unit: MqttStatus::from_u8(level),
        }
    }
    pub fn update_status(&mut self, level: u8) {
        self.level = level;
        self.unit = MqttStatus::from_u8(level);
    }
}

impl LevelUnit for MqttLevelUnit {
    fn msg(&self) -> &'static str {
        self.msg
    }
    fn level(&self) -> u8 {
        self.level
    }
    fn unit(&self) -> &'static str {
        self.unit.to_str()
    }
}

pub trait DurationExt {
    fn to_string(&self) -> String<20>;
}

impl DurationExt for Duration {
    fn to_string(&self) -> String<20> {
        let mut s = String::<20>::new();
        write!(&mut s, "{}", self).unwrap();
        s
    }
}

pub struct DisplayData {
    pub wifi: WifiLevelUnit,
    pub mqtt_client: MqttLevelUnit,
    pub us_since_last_update: Duration,
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
    data: &DisplayData,
) where
    D: AsyncI2c,
{
    // Display time first
    let time_str = data.us_since_last_update.to_string();
    Text::with_baseline(&time_str, Point::zero(), *text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    // Skip lines and display other data
    let font_height = &DISPLAY_FONT.character_size.height;
    let mut y: i32 = (*font_height * 2).try_into().unwrap();

    let wifi_status_str = data.wifi.to_string();
    // HACK: WILL NOT WORK FOR DIFF FONTS
    Text::with_baseline(
        &wifi_status_str,
        Point::new(0, y),
        *text_style,
        Baseline::Top,
    )
    .draw(display)
    .unwrap();

    let mqtt_status_str = data.mqtt_client.to_string();
    y = (*font_height * 4).try_into().unwrap();
    Text::with_baseline(
        &mqtt_status_str,
        Point::new(0, y),
        *text_style,
        Baseline::Top,
    )
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
