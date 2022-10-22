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

impl Server {
    pub fn new(cfg: ServerConfig) -> Self {
        Self {
            // TODO: pad these lines out to 20 characters
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
            &format!("{:<20}", &self.header_l1),
            &format!("{:<20}", &self.header_l2),
            &format!("{:<20}", &self.header_l3),
            &format!("{:<20}", &self.header_l4),
            "",
        ]
        .join("\r\n")
        .into_bytes()
    }

    pub fn s602tt(
        &mut self,
        i: usize,
        product: String,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if i == 0 {
            self.tanks.iter_mut().for_each(|t| {
                t.product = product.clone();
            });
        } else {
            if let None = self.tanks.get_mut(i - 1).map(|t| {
                t.product = product;
            }) {
                Err("tank doesn't exist")?
            }
        }

        // TODO - do real servers only return the changed tanks?
        Ok(self
            .tanks
            .iter()
            .enumerate()
            .fold(
                // TODO: is there actually extra space here?
                // the manual's example response has 3 spaces
                Table::new("{:^}   {:<}")
                    .set_line_end("\r\n")
                    .with_row(Row::from_cells(["TANK", "PRODUCT LABEL"])),
                |acc, (i, curr)| {
                    acc.with_row(Row::from_cells(
                        [
                            format!("{:>2}", i + 1),
                            format!("{:<20}", curr.product.clone()),
                        ]
                        .iter()
                        .cloned(),
                    ))
                },
            )
            .to_string()
            .into_bytes())
    }

    pub fn i20100(&self) -> Vec<u8> {
        self.tanks
            .iter()
            .enumerate()
            .fold(
                Table::new("{:^} {:<} {:>} {:>} {:>} {:>} {:>} {:>}")
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
                            format!("{:<20}", curr.product.clone()),
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
