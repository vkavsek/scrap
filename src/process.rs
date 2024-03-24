use std::ops::RangeInclusive;

use anyhow::Result;
use chrono::Local;
use reqwest::Client;
use tokio::{fs::File, io::AsyncWriteExt, sync::mpsc};

use crate::{info_time, BLOCK_RANGE_LIMIT, EXPECTED_NUM_OF_ENTRIES, FILE_PATH};

use crate::parse::parse_block;
use crate::request::request_block;

pub async fn process_site() -> Result<()> {
    let start_time = Local::now();
    let client = reqwest::Client::new();

    info_time!("Started scraping");

    let block_range_limit = BLOCK_RANGE_LIMIT;
    let block_range = if block_range_limit > 0 {
        1..=block_range_limit
    } else {
        1..=usize::MAX
    };

    let (str_tx, str_rx) = tokio::sync::mpsc::channel(256);
    let collect_handle = tokio::spawn(async move { collect_entries(str_rx).await });

    process_blocks(str_tx, block_range, client).await?;
    info_time!(start_time, "Finished PROCESSING ALL blocks.");

    let res_entries = collect_handle
        .await??
        .into_iter()
        .flat_map(|s| s.into_bytes())
        .collect::<Vec<_>>();

    let local_now = Local::now();
    let mut file = File::create(FILE_PATH).await?;
    file.write_all(&res_entries).await?;
    info_time!(local_now, "Wrote the results to file: {FILE_PATH}");

    Ok(())
}

async fn collect_entries(mut str_rx: mpsc::Receiver<Vec<(String, String)>>) -> Result<Vec<String>> {
    info_time!("Started collecting entries");
    let start_time = Local::now();
    let mut col = Vec::with_capacity(EXPECTED_NUM_OF_ENTRIES);

    while let Some(s) = str_rx.recv().await {
        info_time!("Recieved a new entry: LEN: {}", s.len());
        col.extend_from_slice(&s);
    }

    info_time!(
        start_time,
        "DONE: {} entries, Expected: {EXPECTED_NUM_OF_ENTRIES}",
        col.len()
    );
    let pre_sort_now = Local::now();
    col.sort_unstable();
    let col = col.into_iter().map(|(_, entry)| entry).collect();
    info_time!(pre_sort_now, "Sorted all entries.");

    Ok(col)
}

async fn process_blocks(
    str_tx: mpsc::Sender<Vec<(String, String)>>,
    block_range: RangeInclusive<usize>,
    client: Client,
) -> Result<()> {
    let block_num_init = 0;
    let mut block_to_process = tokio::spawn({
        let client = client.clone();
        async move { request_block(block_num_init, client).await }
    });

    // Range is just two 8 byte numbers on 64-bit architecture - cheap to clone.
    for next_block_num in block_range.clone() {
        let current_block_n = next_block_num - 1;

        let start_block_time = Local::now();
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();

        // Spawn a task that will request process the block requested in previous iteration
        let process_handle = tokio::spawn({
            let str_tx = str_tx.clone();
            async move { parse_block(block_to_process, Some(stop_tx), str_tx).await }
        });

        // Spawn a task that will request the block to be processed in the next iteration.
        block_to_process = tokio::spawn({
            let client = client.clone();
            async move { request_block(next_block_num, client).await }
        });

        // Await processing
        process_handle.await??;
        info_time!(start_block_time, "Processed block {}", current_block_n);

        if stop_rx.try_recv().is_ok() || &next_block_num == block_range.end() {
            // Process the block that was requested in the current (last) iter.
            parse_block(block_to_process, None, str_tx).await?;
            info_time!(start_block_time, "Processed block {}", next_block_num);
            break;
        }
    }
    Ok(())
}
