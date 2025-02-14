use embedded_graphics::{
    mono_font::{MonoFont, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use embedded_hal_async::i2c::I2c as AsyncI2c;
use ssd1306::prelude::DisplaySize128x64;
use ssd1306::{mode::BufferedGraphicsModeAsync, prelude::*, Ssd1306Async};

const OLED_ADDRESS: u8 = 0x3C;

pub fn configure_text_style<'f>(font: &'f MonoFont) -> MonoTextStyle<'f, BinaryColor> {
    MonoTextStyleBuilder::new()
        .font(font)
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
    message: &str,
) where
    D: AsyncI2c,
{
    Text::with_baseline(message, Point::new(0, 16), *text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    display.flush().await.unwrap();
}
