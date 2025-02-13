use embassy_time::{Duration, Timer};
use log::info;

#[embassy_executor::task(pool_size = 2)]
pub async fn say_hi() -> () {
    Timer::after(Duration::from_secs(5)).await;
    info!("Hello from say_hi task!");
}
