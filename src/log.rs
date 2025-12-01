use std::sync::Mutex;
use std::fs::OpenOptions;
use std::io::Write;

pub const MAX_LOGS_BEFORE_FLUSH: usize = 10;

pub static LOG_SERVER_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());
pub static LOG_CLIENT_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());


fn flush_buffer(buffer: &mut Vec<String>, path: &'static str) {
    if buffer.is_empty() {
        return;
    }

    let content = buffer.join("");
    buffer.clear();

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(content.as_bytes());
        let _ = file.flush();
    }
}

pub fn flush_server() {
    let mut buffer = LOG_SERVER_BUFFER.lock().unwrap();
    flush_buffer(&mut buffer, "src/logs/log_server.txt");
}

pub fn flush_client() {
    let mut buffer = LOG_CLIENT_BUFFER.lock().unwrap();
    flush_buffer(&mut buffer, "src/logs/log_client.txt");
}


#[macro_export]
macro_rules! log_s {
    ($($arg:tt)*) => {{
        use $crate::log::{LOG_SERVER_BUFFER, flush_server, MAX_LOGS_BEFORE_FLUSH};
        let ts = chrono::Local::now().format("%d-%m-%y %H.%M.%S").to_string();
        let entry = format!("[{}] {}\n", ts, format!($($arg)*));

        let mut buf = LOG_SERVER_BUFFER.lock().unwrap();
        buf.push(entry);

        if buf.len() >= MAX_LOGS_BEFORE_FLUSH {
            drop(buf);
            flush_server();
        }
    }};
}

#[macro_export]
macro_rules! log_c {
    ($($arg:tt)*) => {{
        use $crate::log::{LOG_CLIENT_BUFFER, flush_client, MAX_LOGS_BEFORE_FLUSH};
        let ts = chrono::Local::now().format("%d-%m-%y %H.%M.%S").to_string();
        let entry = format!("[{}] {}\n", ts, format!($($arg)*));

        let mut buf = LOG_CLIENT_BUFFER.lock().unwrap();
        buf.push(entry);

        if buf.len() >= MAX_LOGS_BEFORE_FLUSH {
            drop(buf);
            flush_client();
        }
    }};
}

pub fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("[LOGGER] Panic occurred: {}", panic_info);
        flush_server();
        flush_client();
    }));
}