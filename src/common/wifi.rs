use embassy_net::{Runner, Stack};
use embassy_time::{Duration, Timer};
use log::{debug, info, warn};

use esp_wifi::wifi::{
    ClientConfiguration, Configuration, WifiController, WifiDevice, WifiStaDevice, WifiState,
};

use core::sync::atomic::{AtomicI8, Ordering};

// Global atomic variable for the current WiFi signal strength
pub static CURRENT_RSSI: AtomicI8 = AtomicI8::new(-100); // Default value when not connected

#[embassy_executor::task]
pub async fn connection_task(
    mut controller: WifiController<'static>,
    ssid: &'static str,
    ssid_password: &'static str,
) {
    info!("Start connection task");
    info!("Device capabilities: {:?}", controller.capabilities());

    'start_conn_loop: loop {
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: ssid.try_into().unwrap(),
                password: ssid_password.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            info!("Starting WiFi...");
            controller.start_async().await.unwrap();
        }

        info!("Attempting connection...");
        match controller.connect_async().await {
            Ok(_) => info!("Connected!"),
            Err(e) => {
                warn!("Connection failed: {e:?}");
                info!("Retrying in 5 secs...");
                Timer::after(Duration::from_secs(5)).await;
                continue 'start_conn_loop;
            }
        }

        // Main polling loop while connected
        'poll_rssi_when_conn: while esp_wifi::wifi::wifi_state() == WifiState::StaConnected {
            if let Ok(scan_res) = controller.scan_n::<10>() {
                for ap in scan_res.0 {
                    // HACK: N=10 is arbitrary, in my case I scan current conn SSID twice
                    if ap.ssid == ssid {
                        let rssi = ap.signal_strength;
                        CURRENT_RSSI.store(rssi, Ordering::Relaxed);
                        info!(
                            "Updated RSSI for '{}': {} dBm, channel={}",
                            ssid, rssi, ap.channel
                        );
                    }
                }
            }

            Timer::after(Duration::from_secs(10)).await;

            if esp_wifi::wifi::wifi_state() == WifiState::StaDisconnected {
                warn!("WiFi Disconnected! Restarting connection...");
                Timer::after(Duration::from_secs(5)).await;
                break 'poll_rssi_when_conn; // Exit loop to retry connection
            }
        }
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) {
    runner.run().await
}

pub async fn wait_for_connection(stack: Stack<'_>) {
    info!("Waiting for link to be up");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            info!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}

pub const fn get_ssid_password() -> &'static str {
    match option_env!("SSID_PASSWORD") {
        Some(password) => password,
        None => "NOT_FOUND",
    }
}

pub fn approx_rssi_to_percent(rssi: &AtomicI8) -> u8 {
    // Load from atomic and constrain
    let constrained_rssi = rssi.load(Ordering::Relaxed).clamp(-90, -30);
    let rssi_f64 = f64::from(constrained_rssi);

    // For linear mapping: y = mx + b
    // At x=-90, y=0 and at x=-30, y=100
    let m = 100.0 / (-30.0 - (-90.0)); // Slope = 100/60 = 5/3
    let b = 100.0 - (m * (-30.0)); // y-intercept = 100 - m*(-30)
    let qual = (m * rssi_f64 + b) as u8;
    debug!(
        "Converted constrained_rssi={} to quality={}%",
        constrained_rssi, qual
    );
    qual.clamp(0, 100)
}
