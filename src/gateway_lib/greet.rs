use log::info;

pub async fn log_init_complete() -> () {
    info!("All configs init and setup completed!");
}

#[embassy_executor::task(pool_size = 1)]
pub async fn log_from_task() -> () {
    info!("Hello from task");
}
