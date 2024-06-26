use precision_pool::constants::*;
use pretty_assertions::assert_eq;
use scrypto::prelude::*;

#[test]
fn test_tick_base_sqrt() {
    assert_eq!(
        TICK_BASE_SQRT,
        pdec!(1.000049998750062496094023416993798697)
    );
}

#[test]
fn test_min_tick() {
    assert_eq!(MIN_TICK, -887272);
}

#[test]
fn test_max_tick() {
    assert_eq!(MAX_TICK, 887272);
}

#[test]
fn test_input_fee_rate_max() {
    assert_eq!(dec!(0.1), INPUT_FEE_RATE_MAX);
}

#[test]
fn test_input_fee_rate_max_between_0_and_1() {
    assert!(dec!(0) <= INPUT_FEE_RATE_MAX && INPUT_FEE_RATE_MAX <= dec!(1));
}

#[test]
fn test_fee_protocol_share_max() {
    assert_eq!(dec!(0.25), FEE_PROTOCOL_SHARE_MAX);
}

#[test]
fn test_fee_protocol_share_max_between_0_and_1() {
    assert!(dec!(0) <= FEE_PROTOCOL_SHARE_MAX && FEE_PROTOCOL_SHARE_MAX <= dec!(1));
}

#[test]
fn test_flash_loan_fee_rate_max() {
    assert_eq!(dec!(0.1), FLASH_LOAN_FEE_RATE_MAX);
}

#[test]
fn test_flash_loan_fee_rate_max_between_0_and_1() {
    assert!(dec!(0) <= FLASH_LOAN_FEE_RATE_MAX && FLASH_LOAN_FEE_RATE_MAX <= dec!(1));
}

#[test]
fn test_hooks_min_remaining_bucket_fraction() {
    assert_eq!(HOOKS_MIN_REMAINING_BUCKET_FRACTION, dec!(0.9));
}
