use csv::Writer;
use std::fs::File;
use std::str;

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
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
    let conf = config::load_config("resources/config.toml").unwrap_or_else(|e| {
        eprintln!("Could not parse config; using default. Error: {}", e);

        config::Config::default()
    });

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

            loop {
                match socket.read_exact(&mut control).await {
                    Ok(n) if n == 0 => return,
                    Ok(_) => (),
                    Err(_) => return,
                };

                let code = match read_utf8_string(&mut socket, 4).await {
                    Ok(s) => s,
                    Err(_) => return,
                };

                let tank = match read_utf8_string(&mut socket, 2).await {
                    Ok(s) => match str::parse::<usize>(&s) {
                        Ok(s) => s,
                        _ => {
                            eprintln!("Tank NaN");
                            return;
                        }
                    },
                    Err(_) => return,
                };

                if control[0] != SOH {
                    eprintln!("not startwith <SOH>");
                    return;
                }

                let mut w = logger.lock().await;
                log(&mut w, source.ip().to_string(), code.to_string());

                let mut resp = server.read().await.build_header(&code);
                resp.push('\r' as u8);
                resp.push('\n' as u8);

                match code.as_ref() {
                    // In-tank inventory
                    "I201" => {
                        resp.append(&mut "IN-TANK INVENTORY\r\n\r\n".to_string().into_bytes());
                        resp.append(&mut server.read().await.i20100(tank));
                        resp.push('\r' as u8);
                        resp.push('\n' as u8);
                        resp.push(ETX);
                    }
                    // Delivery report
                    "I202" => resp = UNRECOGNIZED.to_vec(),
                    // In-tank leak detect report
                    "I203" => resp = UNRECOGNIZED.to_vec(),
                    // Shift report
                    "I204" => resp = UNRECOGNIZED.to_vec(),
                    // In-tank status report
                    "I205" => {
                        resp.append(&mut server.read().await.i205(tank));
                        resp.push('\r' as u8);
                        resp.push('\n' as u8);
                        resp.push(ETX);
                    }
                    // Set print header line
                    "S503" => {
                        let label = match read_utf8_string(&mut socket, 20).await {
                            Ok(s) => s,
                            // If we don't get the expected payload, we don't know
                            // how many bytes we were supposed to read / when the next
                            // message starts, so we need to just die.
                            //
                            // We could also send UNRECOGNIZED here before severing the connection
                            Err(_) => return,
                        };

                        match server.write().await.s503tt(tank, label) {
                            Ok(mut b) => {
                                resp.append(&mut b);
                                resp.push('\r' as u8);
                                resp.push('\n' as u8);
                                resp.push(ETX);
                            }
                            Err(_) => resp = UNRECOGNIZED.to_vec(),
                        }
                    }
                    // Set tank product label
                    "S602" => {
                        resp.append(&mut "TANK PRODUCT LABEL\r\n\r\n".to_string().into_bytes());

                        let product = match read_utf8_string(&mut socket, 20).await {
                            Ok(s) => s,
                            // If we don't get the expected payload, we don't know
                            // how many bytes we were supposed to read / when the next
                            // message starts, so we need to just die.
                            //
                            // We could also send UNRECOGNIZED here before severing the connection
                            Err(_) => return,
                        };

                        match server.write().await.s602tt(tank, product.to_string()) {
                            Ok(mut b) => {
                                resp.append(&mut b);
                                resp.push('\r' as u8);
                                resp.push('\n' as u8);
                                resp.push(ETX);
                            }
                            Err(_) => resp = UNRECOGNIZED.to_vec(),
                        }
                    }
                    _ => resp = UNRECOGNIZED.to_vec(),
                }

                let _ = socket.write_all(&resp).await;
            }
        });
    }
}

async fn read_utf8_string(
    socket: &mut TcpStream,
    n: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf = vec![0; n];
    socket.read_exact(&mut buf).await?;
    Ok(str::from_utf8(&buf)?.to_string())
}
