use scrypto::prelude::*;

pub trait Truncate {
    fn truncate(&self) -> Decimal;
}

impl Truncate for PreciseDecimal {
    fn truncate(&self) -> Decimal {
        self.checked_truncate(RoundingMode::ToNegativeInfinity)
            .unwrap()
    }
}
