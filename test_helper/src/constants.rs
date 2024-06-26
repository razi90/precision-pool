use precision_pool::pool_math::tick_to_price_sqrt;
use lazy_static::lazy_static;
use scrypto::prelude::*;

pub const DEC_5: Decimal = dec!(5);
pub const DEC_10: Decimal = dec!(10);
pub const FEE_RATE: Decimal = dec!("0.1");

pub const MAX_TICK: i32 = 887272;
pub const MIN_TICK: i32 = -MAX_TICK;
pub const TICK_LEFT_BOUND: i32 = -10000;
pub const TICK_LEFT_MIDDLE_BOUND: i32 = 5000;
pub const TICK_RIGHT_MIDDLE_BOUND: i32 = 10000;
pub const TICK_RIGHT_BOUND: i32 = 15000;

lazy_static! {
    pub static ref PRICE_LESSER_LEFT_BOUND_SQRT: PreciseDecimal = pdec!("0.3")
        .checked_sqrt()
        .unwrap();

    // price ~ 0.3678, sqrt(1.0001^-10000) = 0.6065458221578347578405131291196676381010272902296962274902331629
    pub static ref PRICE_LEFT_BOUND_SQRT: PreciseDecimal = tick_to_price_sqrt(TICK_LEFT_BOUND);

    pub static ref PRICE_BETWEEN_LEFT_BOUNDS_SQRT: PreciseDecimal = pdec!("1.0001")
        .checked_sqrt()
        .unwrap();

    // price ~ 1.6486, sqrt(1.0001^5000) = 1.2840093675402745166797149588385944304302866383038001267442213538
    pub static ref PRICE_LEFT_MIDDLE_BOUND_SQRT: PreciseDecimal =
        tick_to_price_sqrt(TICK_LEFT_MIDDLE_BOUND);

    pub static ref PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT: PreciseDecimal = pdec!(2)
        .checked_sqrt()
        .unwrap();

    // price ~ 2.7181, sqrt(1.0001^10000) = 1.648680055931175769628200045451048976844011249465030400193593239
    pub static ref PRICE_RIGHT_MIDDLE_BOUND_SQRT: PreciseDecimal =
        tick_to_price_sqrt(TICK_RIGHT_MIDDLE_BOUND);

    pub static ref PRICE_BETWEEN_RIGHT_BOUNDS_SQRT: PreciseDecimal = pdec!(3)
        .checked_sqrt()
        .unwrap();

    // price ~ 4.4813, sqrt(1.0001^15000) = 2.1169206358924534159037443803110466073595471619988425247370153363
    pub static ref PRICE_RIGHT_BOUND_SQRT: PreciseDecimal = tick_to_price_sqrt(TICK_RIGHT_BOUND);

    pub static ref PRICE_GREATER_RIGHT_BOUND_SQRT: PreciseDecimal = pdec!(5)
        .checked_sqrt()
        .unwrap();
}
