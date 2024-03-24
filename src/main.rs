//! FINAL FRAN SCRAPER
//! MORE WORK IN A SEPARATE REPO
// TODO:
//  -   Document
//  -   Write a server to browse
//  -   Split into logical modules
//  -   Try figuring out how to deal with empty pages.
//  -   Test

use std::sync::Arc;

use anyhow::Result;
use chrono::Local;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{mpsc, oneshot::Sender},
    task::{spawn_blocking, JoinHandle, JoinSet},
};

/// Similar to `info!` macro in tracing.
/// You can pass in the starting time and it will print how long it took from starting time to now.
/// ```
/// info_time!("str {}, {}", 1, 2);
/// let time = Local::now();
/// info_time!(time, "str {}, {}", 1, 2);
/// ```
macro_rules! info_time {
    ($strfm:literal $(,)? $($arg:expr),*) => {{
        let local_now = Local::now();
        let res = format!("{:<30} : {}", local_now, format!($strfm, $($arg),*));
        println!("{}", res);
    }};
    ($time:expr, $strfm:literal $(,)? $($arg:expr),*) => {{
        let local_now = Local::now();
        let run_time = (local_now - $time)
                .num_microseconds()
                .map(|n| n as f64 / 1_000_000.0)
                .unwrap_or(0.0);
        let res = format!("{:<30} : {}\nRUNTIME: {} sec", local_now, format!($strfm, $($arg),*), run_time);
        println!("{}", res);
    }};
}

const PAGES_PER_BLOCK: usize = 50;
const START_PAGE: usize = 1;
const FILE_PATH: &str = "SSKJ_agg.txt";
/// If set to 0 the limit is set to usize::MAX
const BLOCK_RANGE_LIMIT: usize = 0;
const EXPECTED_NUM_OF_ENTRIES: usize = 97669;

#[tokio::main]
async fn main() -> Result<()> {
    let start_time = Local::now();
    process_site().await?;
    info_time!(start_time, "Full program time:");

    Ok(())
}

async fn process_site() -> Result<()> {
    let client = reqwest::Client::new();

    let start_time = Local::now();
    info_time!("Started scraping");

    let block_num_init = 0;
    let mut block_to_process = tokio::spawn({
        let client = client.clone();
        async move { request_block(block_num_init, client).await }
    });

    let block_range_limit = BLOCK_RANGE_LIMIT;
    let block_range_inc = if block_range_limit > 0 {
        1..=block_range_limit
    } else {
        1..=usize::MAX
    };

    let (str_tx, str_rx) = tokio::sync::mpsc::channel(256);

    let collect_handle = tokio::spawn(async move { collect_entries(str_rx).await });

    // Range is just two 8 byte numbers on 64-bit architecture - cheap to clone.
    for next_block_num in block_range_inc.clone() {
        let current_block_n = next_block_num - 1;

        let start_block_time = Local::now();
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();

        // Spawn a task that will request process the block requested in previous iteration
        let process_handle = tokio::spawn({
            let str_tx = str_tx.clone();
            async move { process_block(block_to_process, Some(stop_tx), str_tx).await }
        });

        // Spawn a task that will request the block to be processed in the next iteration.
        block_to_process = tokio::spawn({
            let client = client.clone();
            async move { request_block(next_block_num, client).await }
        });
        info_time!("Spawned BLOCK PROCESSING and REQUESTED NEXT BLOCK");

        // Await processing
        process_handle.await??;
        info_time!(start_block_time, "Processed block {}", current_block_n);

        if stop_rx.try_recv().is_ok() || &next_block_num == block_range_inc.end() {
            // Process the block that was requested in the current (last) iter.
            process_block(block_to_process, None, str_tx).await?;
            info_time!(start_block_time, "Processed block {}", next_block_num);
            break;
        }
    }
    info_time!(start_time, "Finished PROCESSING ALL blocks.");

    let local_now = Local::now();
    let res_entries = collect_handle
        .await??
        .into_iter()
        .flat_map(|s| s.into_bytes())
        .collect::<Vec<_>>();
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

async fn request_block(block_num: usize, client: Client) -> JoinSet<Result<String>> {
    info_time!("Requesting block: {block_num}");
    let mut task_set = JoinSet::new();
    let real_page_num = block_num * PAGES_PER_BLOCK + START_PAGE;
    for page_num in real_page_num..real_page_num + PAGES_PER_BLOCK {
        task_set.spawn({
            // Client uses Arc so we can clone cheaply
            let client = client.clone();

            async move { get_html(client, page_num).await }
        });
    }
    task_set
}

async fn get_html(client: Client, page_num: usize) -> Result<String> {
    // TODO: Errors ?
    let res = client
        .get(format!(
            "https://fran.si/iskanje?page={page_num}&FilteredDictionaryIds=133&View=1&Query=*"
        ))
        .send()
        .await
        .unwrap();
    let html = res.text().await.unwrap();
    Ok(html)
}

async fn process_block(
    block_to_process: JoinHandle<JoinSet<Result<String>>>,
    stop_tx: Option<Sender<()>>,
    str_tx: mpsc::Sender<Vec<(String, String)>>,
) -> Result<()> {
    let mut block_to_process = block_to_process.await?;

    let mut found_empty_page = false;
    while let Some(task) = block_to_process.join_next().await {
        let html = task??;
        // Get all words + qualifiers
        let words = process_html(html.into()).await?;
        if words.is_empty() {
            info_time!("found EMPTY page");
            found_empty_page = true;
            continue;
        }
        str_tx.send(words).await?;
    }

    if found_empty_page {
        if let Some(stop_send) = stop_tx {
            info_time!("sending STOP signal");
            let _ = stop_send.send(());
        }
    }
    Ok(())
}

async fn process_html(html: Arc<String>) -> Result<Vec<(String, String)>> {
    let words_vec = spawn_blocking({
        let html = html.clone();
        move || {
            let doc = Html::parse_document(&html);

            // TODO: HANDLE ERROR:
            let entry_selector = Selector::parse(r#"div[class="list-group-item entry"]"#).unwrap();
            let entry_divs = doc.select(&entry_selector);

            // There shouldn't be more than 30 entries per page.
            let mut words_vec = Vec::with_capacity(30);
            for entry_div in entry_divs {
                let span_selector = Selector::parse("span").unwrap();
                let anchor_selector = Selector::parse("a").unwrap();

                let div = Html::parse_fragment(&entry_div.html());

                let token = div
                    .select(&anchor_selector)
                    .next()
                    .expect("Entry didn't contain an anchor")
                    .inner_html();

                // TODO: HANDLE ERROR:
                let qualifier_selector =
                    Selector::parse(r#"span[data-group="header qualifier"]"#).unwrap();
                let mut qual_text = String::new();
                if let Some(qualifier) = div.select(&qualifier_selector).next() {
                    qual_text = Html::parse_fragment(&qualifier.inner_html())
                        .select(&span_selector)
                        .next()
                        .expect("Qualifier span didn't contain another span!")
                        .inner_html();
                }

                let word_entry = format!("{token:<20}{qual_text:<20}\n");
                words_vec.push((fmt_sortable_entry(&token), word_entry));
            }
            words_vec
        }
    })
    .await?;

    Ok(words_vec)
}

// TODO: Add regex + add other letters (r)
fn fmt_sortable_entry(token: &str) -> String {
    token
        .to_lowercase()
        .replace(['á', 'à', 'â', 'ã', 'ä'], "a")
        .replace(['é', 'è', 'ê', 'ë'], "e")
        .replace(['í', 'ì', 'î', 'ï'], "i")
        .replace(['ó', 'ò', 'ô', 'õ', 'ö'], "o")
        .replace(['ú', 'ù', 'û', 'ü'], "u")
        .replace(['ý', 'ÿ'], "y")
        .replace(['ñ'], "n")
}
