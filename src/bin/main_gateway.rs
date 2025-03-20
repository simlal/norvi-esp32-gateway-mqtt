#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_net::{Config, DhcpConfig, StackResources};
use embassy_time::Timer;

use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::i2c;
use esp_hal::peripherals::Peripherals;
use esp_wifi::{wifi::WifiStaDevice, EspWifiController};
use log::{debug, info};

use espnow_mesh_temp_monitoring_rs::common::wifi::{
    connection_task, get_ssid_password, net_task, wait_for_connection,
};
use espnow_mesh_temp_monitoring_rs::gateway_lib::display::{
    configure_text_style, display_message, DisplayData, MqttLevelUnit, WifiLevelUnit,
};

use ssd1306::{prelude::*, size::DisplaySize128x64, I2CDisplayInterface, Ssd1306Async};

// ****** Arena type heap ****** //
extern crate alloc;
// NOTE: HOW MUCH HEAP REQ?
const HEAP_SIZE: usize = 72 * 1024;
const OLED_ADDRESS: u8 = 0x3C;

async fn allocate_heap() {
    esp_alloc::heap_allocator!(HEAP_SIZE);
}

// ****** RUNTIME static vars ****** //
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // ********** Hardware init and heap ********** //
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals: Peripherals = esp_hal::init(config);
    allocate_heap().await;

    esp_println::logger::init_logger_from_env();
    info!(
        "Initialized hardware and allocated {} KB of pre-defined heap",
        HEAP_SIZE / 1024
    );

    // ********** Embassy Init ********** //
    let timer0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);
    info!("Embassy initialized!");

    // ********** I2C For Display ********** //
    //let mut gpio_16 = gpio::Output::new(peripherals.GPIO16, gpio::Level::Low);
    let i2c_module = i2c::master::I2c::new(peripherals.I2C0, i2c::master::Config::default())
        .unwrap()
        .with_sda(peripherals.GPIO16)
        .with_scl(peripherals.GPIO17)
        .into_async();

    let interface = I2CDisplayInterface::new_custom_address(i2c_module, OLED_ADDRESS);
    let mut display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().await.unwrap();
    let text_style = configure_text_style();

    // ********** DisplayData init ********** //
    let wifi_status_display = WifiLevelUnit {
        msg: "Wifi",
        level: 0,
        unit: "%",
    };
    let mqtt_status_display = MqttLevelUnit::new("MQTT client", 0);
    let mut device_data = DisplayData::new(wifi_status_display, mqtt_status_display);
    info!("Display with basic device data init");
    display_message(&mut display, &text_style, &mut device_data).await;

    // ********** Wifi init ********** //
    // Wifi creds from both config and compile args
    pub const SSID: &str = env!("SSID");
    pub const SSID_PASSWORD: &str = get_ssid_password();
    debug!("ssid={} pw={}", &SSID, &SSID_PASSWORD);

    // controller and device in STA mode
    let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let mut rng = esp_hal::rng::Rng::new(peripherals.RNG);

    let esp_wifi_ctrl = mk_static!(
        EspWifiController<'static>,
        esp_wifi::init(timg0.timer0, rng, peripherals.RADIO_CLK,).unwrap()
    );
    let (sta_device, sta_controller) =
        esp_wifi::wifi::new_with_mode(esp_wifi_ctrl, peripherals.WIFI, WifiStaDevice).unwrap();
    info!("STA device and controller init OK.");

    // Network stack init
    let net_seed = rng.random() as u64 | (rng.random() as u64) << 32;
    let tls_seed = rng.random() as u64 | (rng.random() as u64) << 32;

    let dhcp_config = DhcpConfig::default();
    let config = Config::dhcpv4(dhcp_config);
    debug!(
        "Setting network stack with random seed={} and DHCP with IpV4: {:?}",
        net_seed, config.ipv4
    );

    // Spawn wifi connection tasks to poll for conn and wait for conn
    info!("Spawning connection and network stack tasks...");
    let (stack, runner) = embassy_net::new(
        sta_device,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        net_seed,
    );
    spawner
        .spawn(connection_task(sta_controller, SSID, SSID_PASSWORD))
        .unwrap();
    spawner.spawn(net_task(runner)).unwrap();

    wait_for_connection(stack).await;
    info!("Connection to Wifi '{}' successfull!", SSID);

    info!("All configs init and setup completed!");
    // ********** init end ********** //
    //
    loop {
        display.clear_buffer();
        device_data.mqtt_client.update_status(1);
        display_message(&mut display, &text_style, &mut device_data).await;
        Timer::after_secs(5).await;

        display.clear_buffer();
        device_data.mqtt_client.update_status(2);
        display_message(&mut display, &text_style, &mut device_data).await;
        Timer::after_secs(3).await;
    }
}
