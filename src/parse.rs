use std::sync::Arc;

use chrono::Local;
use scraper::{Html, Selector};
use tokio::{
    sync::{mpsc, oneshot::Sender},
    task::{spawn_blocking, JoinHandle, JoinSet},
};

use crate::{info_time, Error, Result};

/// Accepts a `JoinHandle` containing a `JoinSet` of all the pages in a block and attempts to parse them.
/// If it encounters an empty page it sends out a STOP signal.
/// It sends the parsed pages through a `mpsc:channel` in the form of **(sorting_string, actual_string)**,
/// so that they can be collected and sorted.
pub(crate) async fn parse_block(
    block_to_parse: JoinHandle<JoinSet<Result<String>>>,
    stop_tx: Option<Sender<()>>,
    str_tx: mpsc::Sender<Vec<(String, String)>>,
) -> Result<()> {
    let mut block_to_process = block_to_parse.await?;

    let mut found_empty_page = false;
    while let Some(task) = block_to_process.join_next().await {
        let html = task??;
        // Get all tokens + qualifiers
        let words = parse_html(html.into()).await?;
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

// TODO: TEST with irrelevant divs inserted and just pages with entries
//
/// Attempts to parse the page, extracting the relevant <divs>.
/// Returns a `Vec` of formatted strings: `"{token} | {qualifier}"`.
async fn parse_html(html: Arc<String>) -> Result<Vec<(String, String)>> {
    let words_vec = spawn_blocking({
        let html = html.clone();
        move || -> Result<Vec<(String, String)>> {
            let doc = Html::parse_document(&html);

            // Create selectors.
            let entry_selector = create_selector(r#"div[class="entry"]"#)?;
            let token_selector = create_selector(r#"div[class="token"]"#)?;
            let qual_selector = create_selector(r#"div[class="qual"]"#)?;

            // There shouldn't be more than 30 entries per page.
            let mut words_vec = Vec::with_capacity(30);
            let entry_divs = doc.select(&entry_selector);
            for entry_div in entry_divs {
                let div = Html::parse_fragment(&entry_div.html());

                let token = div
                    .select(&token_selector)
                    .next()
                    .expect("Entry didn't contain a token")
                    .inner_html();

                let word_entry;
                if let Some(qualifier) = div.select(&qual_selector).next() {
                    let qual_text = qualifier.inner_html();
                    word_entry = format!("{:<20} | {:<20}\n", token.trim(), qual_text.trim());
                } else {
                    word_entry = format!("{}\n", token.trim());
                }

                words_vec.push((fmt_sortable_entry(&token), word_entry));
            }
            Ok(words_vec)
        }
    })
    .await??;

    Ok(words_vec)
}

#[inline]
fn create_selector(sel_str: &str) -> Result<Selector> {
    Selector::parse(sel_str).map_err(|_| Error::ParseMissingSelector(sel_str.into()))
}

// TODO: Add regex + add other letters (r)
//
/// Converts letters with accents to letters without accents so that the words can be
/// sorted_correctly.
#[inline]
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
        .replace(['ŕ'], "r")
}
