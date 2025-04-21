//
//! Temperature and analog measurement module for Norvii iIOT AE04
//!
//! Hardware configuration:
//! - 6 analog inputs (A0-A5)
//! - Two ADS1115 ADCs with addresses 0x48 and 0x49
//! - Input mapping:
//!   - A0: ADS1115 0x48 AIN0
//!   - A1: ADS1115 0x48 AIN1
//!   - A2: ADS1115 0x48 AIN2
//!   - A3: ADS1115 0x48 AIN3
//!   - A4: ADS1115 0x49 AIN0
//!   - A5: ADS1115 0x49 AIN1
//! - 4-20mA measurement range

// use ads1x1x::{channel, Ads1x1x, FullScaleRange, TargetAddr};
use core::sync::atomic::{AtomicI16, Ordering};
// use esp_hal::i2c::master::I2c;
// use esp_hal::Async;
use crate::common::rng::SimpleRngU64;
use log::info;

// ****** I2C address for ADS1115 ****** //
// const ADS1115_ADDR: u8 = 0x48;

// Global atomic variable for the current temperature
pub static ESP32_TEMP1: AtomicI16 = AtomicI16::new(0); // Default value when not connected

// pub fn read_temperature(
//     i2c: &mut I2c<'_, Async>,
// ) -> Result<f32, nb::Error<ads1x1x::Error<esp_hal::i2c::master::Error>>> {
//     // Create a temporary I2C instance for ADS1115
//     let peripherals = unsafe { esp_hal::peripherals::Peripherals::steal() };
//     let i2c_config = esp_hal::i2c::master::Config::default();
//
//     // Lower speed for better reliability
//     let mut i2c = match esp_hal::i2c::master::I2c::new(peripherals.I2C1, i2c_config) {
//         Ok(i2c) => i2c
//             .with_sda(peripherals.GPIO16)
//             .with_scl(peripherals.GPIO17)
//             .into_async(),
//         Err(e) => {
//             error!("Failed to create I2C interface");
//             return Err(nb::Error::Other(e));
//         }
//     };
//
//     // Create an instance of the ADS1115 ADC
//     let mut adc = Ads1x1x::new_ads1115(&mut i2c, ads1x1x::TargetAddr::Gnd);
//
//     // Try to set range and read from ADS1115
//     match adc.set_full_scale_range(FullScaleRange::Within4_096V) {
//         Ok(_) => {
//             match adc.read(channel::SingleA0) {
//                 Ok(voltage) => {
//                     // Convert to millivolts and store
//                     let millivolts = (voltage as f32 * 1000.0) as i16;
//                     info!("ADS1115 raw voltage: {} mV", millivolts);
//                     ESP32_TEMP1.store(millivolts, Ordering::Relaxed);
//
//                     // Convert to temperature
//                     let temp = raw_voltage_to_temp(&ESP32_TEMP1);
//                     Ok(temp)
//                 }
//                 Err(e) => {
//                     error!("Failed to read from ADS1115");
//                     return Err(e);
//                 }
//             }
//         }
//         Err(e) => {
//             error!("Failed to configure ADS1115");
//             return Err(e);
//         }
//     }
// }
//
// HACK: Random temp generated from clock
pub fn read_temperature_hack() -> f32 {
    let mut rng = SimpleRngU64::new();
    let voltage_mv = rng.generate_from_range(0, 1000) as i16; // HACK: Random voltage
    ESP32_TEMP1.store(voltage_mv, Ordering::Relaxed);
    let temp = raw_voltage_to_temp(&ESP32_TEMP1);
    info!("HACK: Generated temperature: {} C", temp);
    temp
}

pub fn raw_voltage_to_temp(voltage: &AtomicI16) -> f32 {
    let raw_voltage = voltage.load(Ordering::Relaxed) as f32;
    let temp = (raw_voltage - 0.5) / 100.0; // HACK: PLACEHOLDER LINEAR REG
    info!(
        "Converted Raw voltage: {} to Temperature: {} C",
        raw_voltage, temp
    );
    temp
}
