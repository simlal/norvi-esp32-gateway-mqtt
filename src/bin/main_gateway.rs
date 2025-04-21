#![no_std]
#![no_main]
use core::fmt::Write;
use core::sync::atomic::Ordering;
use embedded_graphics::{mono_font::MonoTextStyle, pixelcolor::BinaryColor};

use embassy_executor::Spawner;
use embassy_net::{tcp::TcpSocket, Config, DhcpConfig, StackResources};
use embassy_time::{Duration, Ticker};
use esp_hal::i2c::master::I2c;
use esp_hal::Async;

use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::i2c;
use esp_hal::peripherals::Peripherals;
use esp_wifi::{wifi::WifiStaDevice, EspWifiController};
use log::{debug, error, info};

use heapless::String;

// MQTT related imports
use rust_mqtt::{
    client::{client::MqttClient, client_config::ClientConfig},
    packet::v5::reason_codes::ReasonCode,
    utils::rng_generator::CountingRng,
};

use espnow_mesh_temp_monitoring_rs::common::wifi::{
    approx_rssi_to_percent, connection_task, get_ssid_password, net_task, wait_for_connection,
    CURRENT_RSSI,
};

use espnow_mesh_temp_monitoring_rs::common::temperature::read_temperature_hack;

use espnow_mesh_temp_monitoring_rs::gateway_lib::display::{
    configure_text_style, display_update_task, DisplayData, MqttLevelUnit, TemperatureLevelUnit,
    WifiLevelUnit, CURRENT_MQTT,
};
// TEST: Test the http requests call with this module
// use espnow_mesh_temp_monitoring_rs::gateway_lib::requests::make_get_request;

use ssd1306::{
    mode::BufferedGraphicsModeAsync, prelude::*, size::DisplaySize128x64, I2CDisplayInterface,
    Ssd1306Async,
};

// ****** Arena type heap ****** //
extern crate alloc;
// NOTE: HOW MUCH HEAP REQ?
const HEAP_SIZE: usize = 72 * 1024;
const OLED_ADDRESS: u8 = 0x3C;
const ADS1115_ADDR: u8 = 0x48;

fn allocate_heap() {
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
async fn scan_i2c_bus() {
    // Get a new peripherals instance
    let peripherals = unsafe { esp_hal::peripherals::Peripherals::steal() };
    let i2c_config = i2c::master::Config::default();

    // Create a temporary I2C instance for scanning
    let mut i2c = i2c::master::I2c::new(peripherals.I2C1, i2c_config)
        .unwrap()
        .with_sda(peripherals.GPIO16)
        .with_scl(peripherals.GPIO17)
        .into_async();

    info!("Scanning I2C bus...");
    let mut devices_found = false;

    for addr in 0..=127u8 {
        if addr == OLED_ADDRESS {
            // Skip the OLED address since we know it's there
            continue;
        }

        let mut data = [0u8; 1];
        if let Ok(_) = i2c.write_read(addr, &[0], &mut data).await {
            info!("Found I2C device at address: 0x{:02X}", addr);
            devices_found = true;

            if addr == ADS1115_ADDR {
                info!("Found ADS1115 at expected address 0x{:02X}", addr);
            }
        }
    }

    if !devices_found {
        error!("No I2C devices found besides OLED! Check connections.");
    }
}

async fn scan_i2c(i2c: &mut I2c<'_, Async>) {
    info!("Scanning I2C bus...");

    let mut devices_found = false;

    // Use the async API to scan for devices
    for addr in 0..=127u8 {
        let write_data = [0u8; 1];
        let mut read_data = [0u8; 1];
        // Just try to read a byte from each address
        match i2c.write_read(addr, &write_data, &mut read_data).await {
            Ok(_) => {
                info!("Found I2C device at address: 0x{:02X}", addr);
                devices_found = true;
            }
            Err(_) => {
                // No device at this address, continue to the next
            }
        }
    }

    if !devices_found {
        error!("No I2C devices found on the bus! Check your connections.");
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // ********** Hardware init and heap ********** //
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals: Peripherals = esp_hal::init(config);
    allocate_heap();

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
    scan_i2c_bus().await;
    let i2c_module = i2c::master::I2c::new(peripherals.I2C0, i2c::master::Config::default())
        .unwrap()
        .with_sda(peripherals.GPIO16)
        .with_scl(peripherals.GPIO17)
        .into_async();

    let interface = I2CDisplayInterface::new_custom_address(i2c_module, OLED_ADDRESS);

    let display = mk_static!(
        Ssd1306Async<
            I2CInterface<I2c<'static, Async>>,
            DisplaySize128x64,
            BufferedGraphicsModeAsync<DisplaySize128x64>,
        >,
        Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode()
    );
    display.init().await.unwrap();
    static TEXT_STYLE: MonoTextStyle<'static, BinaryColor> = configure_text_style();

    // ********** DisplayData init ********** //
    let temp_status_display = TemperatureLevelUnit {
        msg: "Temp",
        level: 0.0,
        unit: "C",
    };
    let wifi_status_display = WifiLevelUnit {
        msg: "Wifi",
        level: 0,
        unit: "%",
    };
    let mqtt_status_display =
        MqttLevelUnit::new("MQTT client", CURRENT_MQTT.load(Ordering::Relaxed));
    let device_data = mk_static!(
        DisplayData,
        DisplayData::new(
            temp_status_display,
            wifi_status_display,
            mqtt_status_display
        )
    );
    info!("Initialized display device, spawning task with ~5s refresh.");
    spawner
        .spawn(display_update_task(display, &TEXT_STYLE, device_data))
        .unwrap();

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
    // let tls_seed = rng.random() as u64 | (rng.random() as u64) << 32;

    let dhcp_config = DhcpConfig::default();
    let config = Config::dhcpv4(dhcp_config);
    debug!(
        "Setting network stack with random seed={} and DHCP with IpV4: {:?}",
        net_seed, config.ipv4
    );

    // Get the mac address for the topic later on
    let mac = sta_device.mac_address();
    let mac_addr_hex = alloc::format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0],
        mac[1],
        mac[2],
        mac[3],
        mac[4],
        mac[5]
    );
    info!("mac address for gateway: {}", &mac_addr_hex);

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

    // Test some request to internet
    // let url = "https://jsonplaceholder.typicode.com/posts/1";
    // make_get_request(stack, tls_seed, url).await;

    // Connecting to Mqtt broker
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mqtt_poll_tick = Duration::from_secs(30);
    let mut mqtt_ticker = Ticker::every(mqtt_poll_tick);

    loop {
        mqtt_ticker.next().await;

        // FIX: SHARE i2c mutex with the temperature read function and the dispaly task

        // HACK: Get a rand reading from the time clock
        let temp = read_temperature_hack();

        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

        // FIX:REFACTOR TO NOT hardcoded IP of host machine where Docker is running
        let host_ip = embassy_net::Ipv4Address::new(192, 168, 68, 108);
        let address = embassy_net::IpAddress::Ipv4(host_ip);

        info!("Connecting to MQTT broker at {}:1883...", address);

        let remote_endpoint = (address, 1883);
        let connection = socket.connect(remote_endpoint).await;
        if let Err(e) = connection {
            error!(
                "connect error: {:?}. Retrying in {}s",
                e,
                mqtt_poll_tick.as_secs()
            );
            continue;
        }
        info!("connected!");

        let mut config = ClientConfig::new(
            rust_mqtt::client::client_config::MqttVersion::MQTTv5,
            CountingRng(20000),
        );
        config.add_max_subscribe_qos(rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1);
        config.add_client_id(&mac_addr_hex);
        config.max_packet_size = 200;
        let mut recv_buffer = [0; 160];
        let mut write_buffer = [0; 160];

        let mut client = MqttClient::<_, 5, _>::new(
            socket,
            &mut write_buffer,
            160,
            &mut recv_buffer,
            160,
            config,
        );

        match client.connect_to_broker().await {
            Ok(()) => {
                info!("Connected to broker!");
                CURRENT_MQTT.store(1, Ordering::Relaxed);
            }
            Err(mqtt_error) => match mqtt_error {
                ReasonCode::NetworkError => {
                    error!(
                        "MQTT Network Error. Retrying in {}s",
                        mqtt_poll_tick.as_secs()
                    );
                    CURRENT_MQTT.store(90, Ordering::Relaxed);
                    continue;
                }
                _ => {
                    error!(
                        "Other MQTT Error: {:?}. Retrying in {}s",
                        mqtt_error,
                        mqtt_poll_tick.as_secs()
                    );
                    CURRENT_MQTT.store(91, Ordering::Relaxed);
                    continue;
                }
            },
        }
        // Get the MAC and make the topic from it
        let gateway_topic = alloc::format!("/readings/gateway/{}", mac_addr_hex);

        // Get the rssi data from the gateway
        let raw_rssi = CURRENT_RSSI.load(Ordering::Relaxed);
        info!("Raw RSSI value: {} dBm", raw_rssi);
        let rssi = approx_rssi_to_percent(&CURRENT_RSSI);
        info!("Current rssi%: {}", rssi);

        // HACK: Create a simple timestamp using uptime, we format it in flask app for now
        let mut uptime_ms = embassy_time::Instant::now().as_millis();
        let mut gateway_data_str: String<128> = String::new();

        write!(
            gateway_data_str,
            "{{\"macAddress\":\"{}\", \"timestamp\":{}, \"rssi\":{:.2}}}",
            mac_addr_hex, uptime_ms, rssi
        )
        .expect("write! failed!");
        info!("Publishing data: {}", gateway_data_str);

        match client
            .send_message(
                &gateway_topic,
                gateway_data_str.as_bytes(),
                rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1,
                true,
            )
            .await
        {
            Ok(()) => {
                info!(
                    "Successfully sent payload to broker on topic={}",
                    &gateway_topic
                )
            }
            Err(mqtt_error) => match mqtt_error {
                ReasonCode::NetworkError => {
                    error!(
                        "MQTT Network Error. Retrying in {}s",
                        mqtt_poll_tick.as_secs()
                    );
                    CURRENT_MQTT.store(90, Ordering::Relaxed);
                    continue;
                }
                _ => {
                    error!(
                        "Other MQTT Error: {:?}. Retrying in {}s",
                        mqtt_error,
                        mqtt_poll_tick.as_secs()
                    );
                    CURRENT_MQTT.store(90, Ordering::Relaxed);
                    continue;
                }
            },
        }

        // HACK: FOR TEMP DATA FROM MESH
        let mesh_sens1_mac = "40:91:51:CB:A4:64";
        let mesh_sens1_topic = alloc::format!("/readings/temperature/{}", mesh_sens1_mac);
        uptime_ms = embassy_time::Instant::now().as_millis();
        let mut mesh_sens1_data_str: String<128> = String::new();
        write!(
            mesh_sens1_data_str,
            "{{\"macAddress\":\"{}\", \"timestamp\":{}, \"temperature\":{:.2}}}",
            mesh_sens1_mac, uptime_ms, temp
        )
        .expect("write! failed!");
        info!("Publishing data: {}", mesh_sens1_data_str);

        match client
            .send_message(
                &mesh_sens1_topic,
                mesh_sens1_data_str.as_bytes(),
                rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1,
                true,
            )
            .await
        {
            Ok(()) => {
                info!(
                    "Successfully sent payload to broker on topic={}",
                    &gateway_topic
                )
            }
            Err(mqtt_error) => match mqtt_error {
                ReasonCode::NetworkError => {
                    error!(
                        "MQTT Network Error. Retrying in {}s",
                        mqtt_poll_tick.as_secs()
                    );
                    CURRENT_MQTT.store(90, Ordering::Relaxed);
                    continue;
                }
                _ => {
                    error!(
                        "Other MQTT Error: {:?}. Retrying in {}s",
                        mqtt_error,
                        mqtt_poll_tick.as_secs()
                    );
                    CURRENT_MQTT.store(90, Ordering::Relaxed);
                    continue;
                }
            },
        }
    }
}
