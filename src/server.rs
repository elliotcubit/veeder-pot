use tabular::{Row, Table};

use crate::tank::Tank;
use chrono::Utc;
use chrono_tz::Tz;

use crate::config::ServerConfig;

pub struct Server {
    // Station header is always 4 lines
    header: [String; 4],

    tanks: Vec<Tank>,
    tc_volume_temp: f32,
    timezone: Tz,
}

impl Server {
    pub fn new(cfg: ServerConfig) -> Self {
        Self {
            header: [
                format!("{:<20}", cfg.header.line1),
                format!("{:<20}", cfg.header.line2),
                format!("{:<20}", cfg.header.line3),
                format!("{:<20}", cfg.header.line4),
            ],
            tanks: cfg.tanks.iter().map(|x| Tank::new(x)).collect(),
            tc_volume_temp: cfg.tc_volume_temp,
            timezone: cfg.timezone,
        }
    }

    pub fn build_header(&self, code: &str) -> Vec<u8> {
        let now = Utc::now().with_timezone(&self.timezone);

        [
            "\x01",
            code,
            now
                .format("%b %e, %Y %l:%M %p")
                .to_string()
                .to_uppercase() // Needed for %b
                .as_str(),
            "",
            &self.header[0],
            &self.header[1],
            &self.header[2],
            &self.header[3],
            "",
        ]
        .join("\r\n")
        .into_bytes()
    }

    pub fn s503tt(
        &mut self,
        i: usize,
        label: String,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if i < 1 || i > 4 {
            Err("invalid line number")?
        }
        self.header[i - 1] = label.clone();
        Ok(format!("# {}: {}", i, &label).into_bytes())
    }

    pub fn s602tt(
        &mut self,
        tank: usize,
        product: String,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if tank > 0 && self.tanks.len() < tank {
            Err("no such tank")?
        }

        self.tanks
            .iter_mut()
            .enumerate()
            .filter(|(i, _)| tank == 0 || *i == tank - 1)
            .for_each(|(_, t)| {
                t.product = product.clone();
            });

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
                    acc.with_row(Row::from_cells([
                        &format!("{:>2}", i + 1),
                        &format!("{:<20}", curr.product),
                    ]))
                },
            )
            .to_string()
            .into_bytes())
    }

    pub fn i20100(&self, tank: usize) -> Vec<u8> {
        self.tanks
            .iter()
            .enumerate()
            .filter(|(i, _)| tank == 0 || *i == tank - 1)
            .fold(
                Table::new("{:^} {:<} {:>} {:>} {:>} {:>} {:>} {:>}")
                    .set_line_end("\r\n")
                    .with_row(Row::from_cells([
                        "TANK",
                        "PRODUCT",
                        "VOLUME",
                        "TC VOLUME",
                        "ULLAGE",
                        "HEIGHT",
                        "WATER",
                        "TEMP",
                    ])),
                |acc, (i, curr)| {
                    acc.with_row(Row::from_cells([
                        &format!("{:>2}", i + 1),
                        &curr.product,
                        &format!("{:.0}", curr.fill()),
                        &format!("{:.0}", curr.tc_volume(self.tc_volume_temp)),
                        &format!("{:.0}", curr.ullage()),
                        &format!("{:.2}", curr.height),
                        &format!("{:.2}", curr.water),
                        &format!("{:.2}", curr.temp),
                    ]))
                },
            )
            .to_string()
            .into_bytes()
    }

    pub fn i205(&self, tank: usize) -> Vec<u8> {
        let mut table = Table::new("{:^}   {:<} {:<}")
            .set_line_end("\r\n")
            .with_row(Row::from_cells(["TANK", "PRODUCT", "STATUS"]));

        self.tanks
            .iter()
            .enumerate()
            .filter(|(i, _)| tank == 0 || *i == tank - 1)
            .for_each(|(i, curr)| {
                table.add_row(Row::from_cells([
                    &format!("{:>2}", i + 1),
                    &curr.product,
                    &curr
                        .warnings
                        .first()
                        .map(|w| w.to_string())
                        .unwrap_or("NORMAL".to_string()),
                ]));

                curr.warnings.iter().skip(1).for_each(|w| {
                    table.add_row(Row::from_cells(["", "", &w.to_string()].iter().cloned()));
                })
            });

        table.to_string().into_bytes()
    }
}
