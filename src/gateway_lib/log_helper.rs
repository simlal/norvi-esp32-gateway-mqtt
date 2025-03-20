use log::info;

#[embassy_executor::task(pool_size = 1)]
pub async fn log_from_task() {
    info!("Hello async from task");
}
