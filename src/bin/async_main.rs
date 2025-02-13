#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use log::info;

extern crate alloc;

#[embassy_executor::task(pool_size = 2)]
async fn say_hi() -> () {
    Timer::after(Duration::from_secs(5)).await;
    info!("hi");
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.2.2

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(72 * 1024);

    esp_println::logger::init_logger_from_env();

    let timer0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");

    // WIFI setup
    //let timer1 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    //let _init = esp_wifi::init(
    //    timer1.timer0,
    //    esp_hal::rng::Rng::new(peripherals.RNG),
    //    peripherals.RADIO_CLK,
    //)
    //.unwrap();

    // TODO: Spawn some tasks
    spawner.spawn(say_hi()).unwrap();
    Timer::after(Duration::from_millis(500)).await;
    spawner.spawn(say_hi()).unwrap();

    loop {
        info!("Hello world!");
        Timer::after(Duration::from_millis(500)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/v0.23.1/examples/src/bin
}
