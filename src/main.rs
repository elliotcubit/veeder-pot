use csv::Writer;
use std::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

use std::str;

fn log(writer: &mut Writer<File>, source_ip: String, code: String) {
    let now_utc: DateTime<Utc> = Utc::now();
    if let Err(err) = writer
        .write_record(&[now_utc.to_rfc3339(), source_ip.clone(), code.clone()])
        .map(|()| writer.flush())
    {
        eprintln!("failed to log {} from {}: {}", code, source_ip, err)
    };
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    let logger = Arc::new(Mutex::new(Writer::from_writer(File::create("log.csv")?)));

    // "If the system receives a command message string containing a
    // function code that it does not recognize, it will respond with
    // a <SOH>9999FF1B<ETX>. The "9999" indicates that the system has
    // not understood the command, while the "FF1B" is the appropriate
    // checksum for the preceding <SOH>9999 string."
    let unrecognized = [1, 57, 57, 57, 57, 70, 70, 49, 66, 3];

    loop {
        let (mut socket, source) = listener.accept().await?;
        let logger = Arc::clone(&logger);

        tokio::spawn(async move {
            let mut control = [0; 1];
            let mut raw_code = [0; 6];

            loop {
                match socket.read_exact(&mut control).await {
                    Ok(n) if n == 0 => return,
                    Ok(_) => (),
                    Err(_) => return,
                };

                match socket.read_exact(&mut raw_code).await {
                    Ok(n) if n == 0 => return,
                    Ok(_) => (),
                    Err(_) => return,
                };

                if control[0] != 1 {
                    eprintln!("not startwith ^A");
                    return;
                }

                let code = match str::from_utf8(&raw_code) {
                    Ok(s) => s,
                    _ => {
                        eprintln!("Invalid UTF-8");
                        return;
                    }
                };

                match code {
                    "I20100" => {
                        let mut w = logger.lock().await;
                        log(&mut w, source.ip().to_string(), code.to_string());
                        eprintln!("IN-TANK INVENTORY");
                        handle_i20100(&mut socket);
                    }
                    _ => {
                        // Since incoming packets do not declare the length of
                        // their data segment, if we don't recognize a command,
                        // we must sever the connection.
                        let mut w = logger.lock().await;
                        log(&mut w, source.ip().to_string(), code.to_string());

                        let _ = socket.write_all(&unrecognized).await;
                        eprintln!("UNRECOGNIZED");
                        return;
                    }
                }
            }
        });
    }
}

fn handle_i20100(_sock: &mut TcpStream) {}
