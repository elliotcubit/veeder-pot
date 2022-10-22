use csv::Writer;
use std::fs::File;
use std::str;

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};

use chrono::{DateTime, Utc};

mod config;
mod server;
mod tank;

use server::Server;

const SOH: u8 = 1;
const ETX: u8 = 3;

// "If the system receives a command message string containing a
// function code that it does not recognize, it will respond with
// a <SOH>9999FF1B<ETX>. The "9999" indicates that the system has
// not understood the command, while the "FF1B" is the appropriate
// checksum for the preceding <SOH>9999 string."
const UNRECOGNIZED: [u8; 10] = [SOH, 57, 57, 57, 57, 70, 70, 49, 66, ETX];

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
    let conf =
        config::load_config("resources/config.toml").unwrap_or_else(|_| config::Config::default());

    let listener = TcpListener::bind(conf.addr).await?;

    let mut writer = Writer::from_writer(File::create(conf.log_file)?);

    writer
        .write_record(&["time", "source_ip", "command"])
        .map(|()| writer.flush())??;

    let logger = Arc::new(Mutex::new(writer));
    let server = Arc::new(RwLock::new(Server::new(conf.server)));

    loop {
        let (mut socket, source) = listener.accept().await?;
        let logger = Arc::clone(&logger);
        let server = Arc::clone(&server);

        tokio::spawn(async move {
            let mut control = [0; 1];
            let mut raw_code = [0; 4];
            let mut raw_tank = [0; 2];

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

                match socket.read_exact(&mut raw_tank).await {
                    Ok(n) if n == 0 => return,
                    Ok(_) => (),
                    Err(_) => return,
                };

                if control[0] != SOH {
                    eprintln!("not startwith <SOH>");
                    return;
                }

                let code = match str::from_utf8(&raw_code) {
                    Ok(s) => s,
                    _ => {
                        eprintln!("Invalid UTF-8");
                        return;
                    }
                };

                let utf8_tank = match str::from_utf8(&raw_tank) {
                    Ok(s) => s,
                    _ => {
                        eprintln!("Invalid tank");
                        return;
                    }
                };

                let tank = match str::parse::<usize>(utf8_tank) {
                    Ok(s) => s,
                    _ => {
                        eprintln!("Tank NaN");
                        return;
                    }
                };

                let mut w = logger.lock().await;
                log(&mut w, source.ip().to_string(), code.to_string());

                let mut resp = server.read().await.build_header(code);
                resp.push('\r' as u8);
                resp.push('\n' as u8);

                match code {
                    // In-tank inventory
                    "I201" => {
                        resp.append(&mut "IN-TANK INVENTORY\r\n\r\n".to_string().into_bytes());
                        resp.append(&mut server.read().await.i20100());
                    }
                    // Delivery report
                    "I202" => resp = UNRECOGNIZED.to_vec(),
                    // In-tank leak detect report
                    "I203" => resp = UNRECOGNIZED.to_vec(),
                    // Shift report
                    "I204" => resp = UNRECOGNIZED.to_vec(),
                    // In-tank status report
                    "I205" => resp = UNRECOGNIZED.to_vec(),
                    // Set print header line
                    "S503" => {
                        // The manual says the new label must be 20 chars...!?
                        let mut raw_label = [0; 20];

                        match socket.read_exact(&mut raw_label).await {
                            Ok(n) if n == 0 => return,
                            Ok(_) => (),
                            Err(_) => return,
                        };

                        let label = match str::from_utf8(&raw_label) {
                            Ok(s) => s,
                            _ => {
                                eprintln!("Invalid UTF-8");
                                return;
                            }
                        };

                        match server.write().await.s503tt(tank, label.to_string()) {
                            Ok(mut b) => resp.append(&mut b),
                            Err(_) => resp = UNRECOGNIZED.to_vec(),
                        }
                    }
                    // Set tank product label
                    "S602" => {
                        resp.append(&mut "TANK PRODUCT LABEL\r\n\r\n".to_string().into_bytes());

                        // The manual says the new label must be 20 chars...!?
                        let mut raw_product = [0; 20];

                        match socket.read_exact(&mut raw_product).await {
                            Ok(n) if n == 0 => return,
                            Ok(_) => (),
                            Err(_) => return,
                        };

                        let product = match str::from_utf8(&raw_product) {
                            Ok(s) => s,
                            _ => {
                                eprintln!("Invalid UTF-8");
                                return;
                            }
                        };

                        match server.write().await.s602tt(tank, product.to_string()) {
                            Ok(mut b) => resp.append(&mut b),
                            Err(_) => resp = UNRECOGNIZED.to_vec(),
                        }
                    }
                    _ => resp = UNRECOGNIZED.to_vec(),
                }

                resp.push('\r' as u8);
                resp.push('\r' as u8);
                resp.push(ETX);

                let _ = socket.write_all(&resp).await;
            }
        });
    }
}
