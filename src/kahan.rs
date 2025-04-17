#[derive(Default)]
pub struct KahanSum {
    sum: f64,
    c: f64,
}

impl KahanSum {
    pub fn new() -> Self {
        Self { sum: 0.0, c: 0.0 }
    }

    pub fn add(&mut self, value: f64) {
        let y = value - self.c;
        let t = self.sum + y;
        self.c = (t - self.sum) - y;
        self.sum = t;
    }

    pub fn sub(&mut self, value: f64) {
        self.add(-value);
    }

    pub fn get(&self) -> f64 {
        self.sum
    }
}
