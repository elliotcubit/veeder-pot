use csv::Writer;
use std::fs::File;
use std::str;

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use chrono::{DateTime, Utc};

mod config;
mod server;
mod tank;

use server::{Server, SOH};

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
    let server = Arc::new(Server::new(conf.server));

    loop {
        let (mut socket, source) = listener.accept().await?;
        let logger = Arc::clone(&logger);
        let server = Arc::clone(&server);

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

                let mut w = logger.lock().await;
                log(&mut w, source.ip().to_string(), code.to_string());

                let _ = socket.write_all(&server.resp(code)).await;
            }
        });
    }
}
