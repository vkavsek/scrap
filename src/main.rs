use chrono::Local;
use scrap::{info_time, process::process_site, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let start_time = Local::now();
    process_site().await?;
    info_time!(start_time, "Full program time:");

    Ok(())
}
