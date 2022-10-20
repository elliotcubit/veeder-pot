const CUBIC_INCH_TO_GALLON: f32 = 0.004329;

pub struct Tank {
    pub product: String,
    pub height: f32,

    // In gallons
    pub water: f32,
    pub temp: f32,

    shape: HorizontalCylinder,
}

impl Tank {
    pub fn new(
        product: String,
        height: f32,
        water: f32,
        temp: f32,
        shape: HorizontalCylinder,
    ) -> Self {
        Self {
            product,
            height,
            water,
            temp,
            shape,
        }
    }

    pub fn tc_volume(&self, tc_volume_temp: f32) -> f32 {
        tc_volume_temp * self.shape.fill(self.height) / self.temp
    }

    pub fn ullage(&self) -> f32 {
        // Subtract the water from ullage... the water needs to be accounted for somewhere.
        // I'm not sure how the water level is measured, but I guess we will treat
        // volume/height/fill as the volume of only gasoline, and water as somehow "magically"
        // acquired through separate means. Since the water level should always be very low
        // (zero is a reasonable value), it shouldn't matter.
        self.shape.volume() - self.shape.fill(self.height) - self.water
    }

    pub fn fill(&self) -> f32 {
        self.shape.fill(self.height)
    }
}

pub struct HorizontalCylinder {
    // Dimensions in inches
    length: f32,
    diameter: f32,
}

impl HorizontalCylinder {
    pub fn new(length: f32, diameter: f32) -> Self {
        Self { length, diameter }
    }
    // Returned in gallons
    fn volume(&self) -> f32 {
        let radius = self.diameter / 2.0;
        std::f32::consts::PI * radius * radius * self.length * CUBIC_INCH_TO_GALLON
    }

    // fill returns how much of the tank is filled, in gallons, given
    // how far from the bottom the liquid rises in inches.
    fn fill(&self, height: f32) -> f32 {
        let radius = self.diameter / 2.0;
        if height > radius {
            self.volume() - self.fill(2.0 * radius - height)
        } else {
            let m = radius - height;
            let theta = 2.0 * (m / radius).acos();
            let a = 0.5 * radius * radius * (theta - theta.sin());
            a * self.length * CUBIC_INCH_TO_GALLON
        }
    }
}
