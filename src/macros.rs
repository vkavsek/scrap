/// Similar to `info!` macro in tracing.
/// You can pass in the starting time and it will print how long it took from starting time to now.
/// ```
/// info_time!("str {}, {}", 1, 2);
/// let time = Local::now();
/// info_time!(time, "str {}, {}", 1, 2);
/// ```
#[macro_export]
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
