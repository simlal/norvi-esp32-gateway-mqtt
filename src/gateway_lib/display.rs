use core::fmt::Write;
use core::sync::atomic::{AtomicU8, Ordering};

use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{
    mono_font::{ascii, MonoFont, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use esp_hal::i2c::master::I2c;
use esp_hal::Async;
use heapless::String;
use ssd1306::prelude::DisplaySize128x64;
use ssd1306::{mode::BufferedGraphicsModeAsync, prelude::*, Ssd1306Async};

use crate::common::temperature::{raw_voltage_to_temp, ESP32_TEMP1};
use crate::common::wifi::{approx_rssi_to_percent, CURRENT_RSSI};

const DISPLAY_FONT: MonoFont = ascii::FONT_5X8;
pub static CURRENT_MQTT: AtomicU8 = AtomicU8::new(0); // init as offline=0

// *** Temperature for display *** //

pub trait FloatLevelUnit {
    fn msg(&self) -> &'static str;
    fn level(&self) -> f32;
    fn unit(&self) -> &'static str;

    // Max of 12 + 6 + 4 chars for level and unit
    fn to_string(&self) -> String<24> {
        let mut s = String::<24>::new();

        // Manual formatting for floating point
        let value = self.level();
        let integer = value as i32;
        let decimal = ((value - integer as f32).abs() * 10.0) as u32; // One decimal place

        // Format message (12 chars max)
        let msg = if self.msg().len() <= 12 {
            self.msg()
        } else {
            &self.msg()[..12]
        };

        // Format unit (4 chars max)
        let unit = if self.unit().len() <= 4 {
            self.unit()
        } else {
            &self.unit()[..4]
        };

        // Avoid using floating point format specifiers
        if value < 0.0 {
            // Handle negative values
            let _ = write!(&mut s, "{:12} -{}.{} {}", msg, integer.abs(), decimal, unit);
        } else {
            let _ = write!(&mut s, "{:12} {}.{} {}", msg, integer, decimal, unit);
        }

        s
    }
}

pub struct TemperatureLevelUnit {
    pub msg: &'static str,
    pub level: f32,
    pub unit: &'static str,
}

impl TemperatureLevelUnit {
    pub fn new(msg: &'static str, level: f32, unit: &'static str) -> TemperatureLevelUnit {
        TemperatureLevelUnit { msg, level, unit }
    }
    pub fn set_level(&mut self, level: f32) {
        self.level = level;
    }
}

impl FloatLevelUnit for TemperatureLevelUnit {
    fn msg(&self) -> &'static str {
        self.msg
    }
    fn level(&self) -> f32 {
        self.level
    }
    fn unit(&self) -> &'static str {
        self.unit
    }
}

// *** Wifi for display *** //
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

// *** MQTT status for display *** //
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
    fn to_string_ms(&self) -> String<20>;
}

impl DurationExt for Duration {
    fn to_string_ms(&self) -> String<20> {
        let duration_as_secs = self.as_millis() as f64 / 1000.0;
        let mut s = String::<20>::new();
        write!(&mut s, "{} s", duration_as_secs).unwrap();
        s
    }
}

// *** DisplayData structure for OLED *** //
pub struct DisplayData {
    pub temperature: TemperatureLevelUnit,
    pub wifi: WifiLevelUnit,
    pub mqtt_client: MqttLevelUnit,
    pub last_update_time: Instant,
    pub last_update_duration: Duration,
}

impl DisplayData {
    pub fn new(
        temperature: TemperatureLevelUnit,
        wifi: WifiLevelUnit,
        mqtt_client: MqttLevelUnit,
    ) -> DisplayData {
        DisplayData {
            temperature,
            wifi,
            mqtt_client,
            last_update_time: Instant::now(),
            last_update_duration: Duration::from_secs(0),
        }
    }
    fn perform_time_update(&mut self) {
        self.last_update_duration = Instant::now()
            .checked_duration_since(self.last_update_time)
            .unwrap_or(Duration::from_secs(0));
        self.last_update_time = Instant::now();
    }
}

pub const fn configure_text_style() -> MonoTextStyle<'static, BinaryColor> {
    MonoTextStyleBuilder::new()
        .font(&DISPLAY_FONT)
        .text_color(BinaryColor::On)
        .build()
}

#[embassy_executor::task]
pub async fn display_update_task(
    display: &'static mut Ssd1306Async<
        I2CInterface<I2c<'static, Async>>, // Use concrete type instead of generic parameter
        DisplaySize128x64,
        BufferedGraphicsModeAsync<DisplaySize128x64>,
    >,
    text_style: &'static MonoTextStyle<'static, BinaryColor>,
    dev_data: &'static mut DisplayData,
) {
    loop {
        display.clear_buffer();

        // Display time first
        dev_data.perform_time_update();
        let time_str = dev_data.last_update_duration.to_string_ms();
        Text::with_baseline(&time_str, Point::zero(), *text_style, Baseline::Top)
            .draw(display)
            .unwrap();

        // HACK: WILL NOT WORK FOR DIFF FONTS
        // Skip lines and display other data
        let font_height = &DISPLAY_FONT.character_size.height;
        let mut y: i32 = (*font_height * 2).try_into().unwrap();

        // Display temperature data
        dev_data.temperature.level = raw_voltage_to_temp(&ESP32_TEMP1);
        let temperature_str = dev_data.temperature.to_string();
        Text::with_baseline(
            &temperature_str,
            Point::new(0, y),
            *text_style,
            Baseline::Top,
        )
        .draw(display)
        .unwrap();

        // Format and update device data
        dev_data.wifi.level = approx_rssi_to_percent(&CURRENT_RSSI);
        let wifi_status_str = dev_data.wifi.to_string();
        y = (*font_height * 4).try_into().unwrap();
        Text::with_baseline(
            &wifi_status_str,
            Point::new(0, y),
            *text_style,
            Baseline::Top,
        )
        .draw(display)
        .unwrap();

        dev_data
            .mqtt_client
            .update_status(CURRENT_MQTT.load(Ordering::Relaxed));
        let mqtt_status_str = dev_data.mqtt_client.to_string();
        y = (*font_height * 6).try_into().unwrap();
        Text::with_baseline(
            &mqtt_status_str,
            Point::new(0, y),
            *text_style,
            Baseline::Top,
        )
        .draw(display)
        .unwrap();

        if let Err(e) = display.flush().await {
            log::error!("Display flush error: {:?}", e);
        }

        Timer::after_secs(5).await;
    }
}
