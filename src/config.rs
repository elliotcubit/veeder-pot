use serde::Deserialize;

use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use std::error::Error;

pub fn load_config(fname: &str) -> Result<Config, Box<dyn Error>> {
    let file = File::open(fname)?;
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

#[derive(Deserialize)]
pub struct HeaderConfig {
    pub line1: String,
    pub line2: String,
    pub line3: String,
    pub line4: String,
}

#[derive(Deserialize)]
pub struct ShapeConfig {
    pub length: f32,
    pub diameter: f32,
}

#[derive(Deserialize)]
pub struct TankConfig {
    pub product: String,
    pub height: f32,
    pub water: f32,
    pub temp: f32,

    pub shape: ShapeConfig,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub header: HeaderConfig,

    pub tanks: Vec<TankConfig>,

    pub tc_volume_temp: f32,
}

#[derive(Deserialize)]
pub struct Config {
    pub addr: String,
    pub log_file: String,
    pub server: ServerConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:10001".to_string(),
            log_file: "log.csv".to_string(),
            server: ServerConfig {
                header: HeaderConfig {
                    line1: "WENDYS BP".to_string(),
                    line2: "24 NIGHT INN AVE.".to_string(),
                    line3: "ATLANTA,GA. 30301".to_string(),
                    line4: "404-308-9102".to_string(),
                },
                tanks: vec![
                    TankConfig {
                        product: "UNLEAD".to_string(),
                        height: 51.95,
                        water: 5.48,
                        temp: 56.46,
                        shape: ShapeConfig {
                            length: 251.184,
                            diameter: 120.0,
                        },
                    },
                    TankConfig {
                        product: "PREMIUM".to_string(),
                        height: 36.2,
                        water: 2.1,
                        temp: 55.70,
                        shape: ShapeConfig {
                            length: 251.184,
                            diameter: 120.0,
                        },
                    },
                    TankConfig {
                        product: "DIESEL".to_string(),
                        height: 47.6,
                        water: 0.0,
                        temp: 58.45,
                        shape: ShapeConfig {
                            length: 251.184,
                            diameter: 120.0,
                        },
                    },
                ],
                tc_volume_temp: 60.00,
            },
        }
    }
}
