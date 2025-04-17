use std::ops::{Add, AddAssign};

/// Local implementation of summator with protection against catastrophic cancellation,
/// proposed by Kahan and improved by Neumaier.
///
/// We could also use `accurate::sum::Neumaier` or even `accurate::sum::Klein`,
/// but with a trade of for efficiency, and since performance is critical I'm proposing
/// simple approach.
#[derive(Default)]
pub struct NeumaierSum {
    s: f64,
    c: f64,
}

impl NeumaierSum {
    pub fn sum(&self) -> f64 {
        self.s + self.c
    }
}

impl From<f64> for NeumaierSum {
    fn from(value: f64) -> Self {
        Self { s: value, c: 0.0 }
    }
}

impl AddAssign<f64> for NeumaierSum {
    fn add_assign(&mut self, rhs: f64) {
        let (s, c) = neumaier_sum(self.s, rhs);

        self.s = s;
        self.c += c;
    }
}

impl Add<f64> for NeumaierSum {
    type Output = Self;

    fn add(mut self, rhs: f64) -> Self::Output {
        self += rhs;
        self
    }
}

impl Add for NeumaierSum {
    type Output = NeumaierSum;

    fn add(self, rhs: Self) -> Self::Output {
        let (s, c1) = neumaier_sum(self.s, rhs.s);
        let (c, _) = neumaier_sum(self.c + c1, rhs.c);
        Self { s, c }
    }
}

#[inline]
fn neumaier_sum(a: f64, b: f64) -> (f64, f64) {
    if a.abs() >= b.abs() {
        kahan_sum(a, b)
    } else {
        kahan_sum(b, a)
    }
}

#[inline]
fn kahan_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let c = (a - s) + b;
    (s, c)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_neumaier_add_assign() {
        let mut s: NeumaierSum = 0.0.into();
        s += 1e200;
        s += 0.1;
        s += 0.2;
        s += 0.3;
        s += -1e200;
        assert!((0.6f64 - s.sum()).abs() < 1e-15);
    }

    #[test]
    fn test_neumaier_add() {
        let s = NeumaierSum::from(0.0) + 1e200 + 0.1 + 0.2 + 0.3 + (-1e200);
        assert!((0.6f64 - s.sum()).abs() < 1e-15);
    }
}
