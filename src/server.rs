use tabular::{Row, Table};

use chrono::{DateTime, Utc};

pub struct Server {
    // Station header is always 4 lines
    header_l1: String,
    header_l2: String,
    header_l3: String,
    header_l4: String,

    tanks: Vec<Tank>,
    tc_volume_temp: u32,
}

pub const SOH: u8 = 1;
const ETX: u8 = 3;

// "If the system receives a command message string containing a
// function code that it does not recognize, it will respond with
// a <SOH>9999FF1B<ETX>. The "9999" indicates that the system has
// not understood the command, while the "FF1B" is the appropriate
// checksum for the preceding <SOH>9999 string."
const UNRECOGNIZED: [u8; 10] = [SOH, 57, 57, 57, 57, 70, 70, 49, 66, ETX];

pub struct Tank {
    product: String,
    volume: u32,
    capacity: u32,
    height: f64,
    water: f64,
    temp: f64,
}

impl Tank {
    pub fn new() -> Self {
        Self {
            product: "UNLEAD".to_string(),
            volume: 3107,
            capacity: 12300,
            height: 51.95,
            water: 5.48,
            temp: 56.46,
        }
    }

    fn tc_volume(&self, tc_volume_temp: u32) -> u32 {
        (f64::from(tc_volume_temp) * (f64::from(self.volume) / self.temp)).round() as u32
    }

    fn ullage(&self) -> u32 {
        (f64::from(self.capacity) - f64::from(self.volume) - self.water).round() as u32
    }
}

impl Server {
    pub fn new() -> Self {
        Self {
            // This is templated from a real example in the wild; information anonymized.
            header_l1: "WENDYS BP".to_string(),
            header_l2: "24 NIGHT INN AVE.".to_string(),
            header_l3: "ATLANTA,GA. 30301".to_string(),
            header_l4: "404-308-9102".to_string(),
            tanks: vec![Tank::new()],
            // They ship like this
            tc_volume_temp: 60,
        }
    }

    pub fn resp(&self, code: &str) -> Vec<u8> {
        let mut resp = self.build_header(code);
        resp.push('\r' as u8);
        resp.push('\n' as u8);

        match code {
            // In-tank inventory
            "I20100" => resp.append(&mut self.payload_i20100()),
            // Delivery report
            "I20200" => resp = UNRECOGNIZED.to_vec(),
            // In-tank leak detect report
            "I20300" => resp = UNRECOGNIZED.to_vec(),
            // Shift report
            "I20400" => resp = UNRECOGNIZED.to_vec(),
            // In-tank status report
            "I20500" => resp = UNRECOGNIZED.to_vec(),
            // Set tank product label
            // TODO - will need to parse TT portion
            "S60200" => resp = UNRECOGNIZED.to_vec(),
            _ => resp = UNRECOGNIZED.to_vec(),
        }

        resp.push('\r' as u8);
        resp.push('\r' as u8);
        resp.push(ETX);

        resp
    }

    fn build_header(&self, code: &str) -> Vec<u8> {
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

    fn payload_i20100(&self) -> Vec<u8> {
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
                            curr.volume.to_string(),
                            curr.tc_volume(self.tc_volume_temp).to_string(),
                            curr.ullage().to_string(),
                            curr.height.to_string(),
                            curr.water.to_string(),
                            curr.temp.to_string(),
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
