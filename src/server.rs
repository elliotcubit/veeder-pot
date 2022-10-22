use tabular::{Row, Table};

use crate::tank::Tank;
use chrono::{DateTime, Utc};

use crate::config::ServerConfig;

pub struct Server {
    // Station header is always 4 lines
    header_l1: String,
    header_l2: String,
    header_l3: String,
    header_l4: String,

    tanks: Vec<Tank>,
    tc_volume_temp: f32,
}

pub const SOH: u8 = 1;
pub const ETX: u8 = 3;

// "If the system receives a command message string containing a
// function code that it does not recognize, it will respond with
// a <SOH>9999FF1B<ETX>. The "9999" indicates that the system has
// not understood the command, while the "FF1B" is the appropriate
// checksum for the preceding <SOH>9999 string."
pub const UNRECOGNIZED: [u8; 10] = [SOH, 57, 57, 57, 57, 70, 70, 49, 66, ETX];

impl Server {
    pub fn new(cfg: ServerConfig) -> Self {
        Self {
            header_l1: cfg.header.line1,
            header_l2: cfg.header.line2,
            header_l3: cfg.header.line3,
            header_l4: cfg.header.line4,
            tanks: cfg.tanks.iter().map(|x| Tank::new(x)).collect(),
            tc_volume_temp: cfg.tc_volume_temp,
        }
    }

    pub fn build_header(&self, code: &str) -> Vec<u8> {
        // TODO use "local" time, not UTC
        let now_utc: DateTime<Utc> = Utc::now();
        [
            "\x01",
            code,
            now_utc
                .format("%b %e, %Y %l:%M %p")
                .to_string()
                .to_uppercase() // Needed for %b
                .as_str(),
            "",
            &self.header_l1,
            &self.header_l2,
            &self.header_l3,
            &self.header_l4,
            "",
        ]
        .join("\r\n")
        .into_bytes()
    }

    pub fn payload_i20100(&self) -> Vec<u8> {
        self.tanks
            .iter()
            .enumerate()
            .fold(
                Table::new("{:^} {:<}             {:>} {:>} {:>} {:>} {:>} {:>}")
                    .set_line_end("\r\n")
                    .with_row(Row::from_cells(
                        [
                            "TANK",
                            "PRODUCT",
                            "VOLUME",
                            "TC VOLUME",
                            "ULLAGE",
                            "HEIGHT",
                            "WATER",
                            "TEMP",
                        ]
                        .iter()
                        .cloned(),
                    )),
                |acc, (i, curr)| {
                    acc.with_row(Row::from_cells(
                        [
                            format!("{:>2}", i + 1),
                            curr.product.clone(),
                            format!("{:.0}", curr.fill()),
                            format!("{:.0}", curr.tc_volume(self.tc_volume_temp)),
                            format!("{:.0}", curr.ullage()),
                            format!("{:.2}", curr.height),
                            format!("{:.2}", curr.water),
                            format!("{:.2}", curr.temp),
                        ]
                        .iter()
                        .cloned(),
                    ))
                },
            )
            .to_string()
            .into_bytes()
    }
}
