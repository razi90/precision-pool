use scrypto::prelude::*;

pub const TICK_BASE_SQRT: PreciseDecimal = pdec!(1.000049998750062496094023416993798697);

pub const MAX_TICK: i32 = 887272;
pub const MIN_TICK: i32 = -MAX_TICK;

pub const MAX_LIQUIDITY: PreciseDecimal = pdec!(3138668841663005800034);

pub const INPUT_FEE_RATE_MAX: Decimal = dec!(0.1);
pub const FEE_PROTOCOL_SHARE_MAX: Decimal = dec!(0.25);
pub const FLASH_LOAN_FEE_RATE_MAX: Decimal = dec!(0.1);
pub const HOOKS_MIN_REMAINING_BUCKET_FRACTION: Decimal = dec!(0.9);

pub const DIVISIBILITY_UNITS: [Decimal; 19] = [
    dec!(1),
    dec!(0.1),
    dec!(0.01),
    dec!(0.001),
    dec!(0.0001),
    dec!(0.00001),
    dec!(0.000001),
    dec!(0.0000001),
    dec!(0.00000001),
    dec!(0.000000001),
    dec!(0.0000000001),
    dec!(0.00000000001),
    dec!(0.000000000001),
    dec!(0.0000000000001),
    dec!(0.00000000000001),
    dec!(0.000000000000001),
    dec!(0.0000000000000001),
    dec!(0.00000000000000001),
    dec!(0.000000000000000001),
];
