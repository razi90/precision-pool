use crate::constants::*;
use common::utils::assert_fee_rate_within_bounds;
use scrypto::prelude::*;

/// Sort two buckets deterministically by their resource address
///
/// # Arguments
/// * `a_bucket`: First bucket to sort
/// * `b_bucket`: Second bucket to sort
///
/// # Returns
/// A tuple consisting of the sorted buckets
pub fn sort_buckets(a_bucket: Bucket, b_bucket: Bucket) -> (Bucket, Bucket) {
    if a_bucket.resource_address() < b_bucket.resource_address() {
        return (a_bucket, b_bucket);
    }
    (b_bucket, a_bucket)
}

pub fn assert_input_fee_rate_is_valid(input_fee_rate: Decimal) {
    assert_fee_rate_within_bounds(input_fee_rate, INPUT_FEE_RATE_MAX, "input fee rate");
}

pub fn assert_flash_loan_fee_rate_is_valid(flash_loan_fee_rate: Decimal) {
    assert_fee_rate_within_bounds(
        flash_loan_fee_rate,
        FLASH_LOAN_FEE_RATE_MAX,
        "flash loan fee rate",
    );
}

pub fn assert_hooks_bucket_output(
    input_amount: Decimal,
    output_amount: Decimal,
    hook_type_name: &str,
) {
    assert!(
        input_amount * HOOKS_MIN_REMAINING_BUCKET_FRACTION <= output_amount,
        "{} hooks took more tokens than the allowed limit of 10%",
        hook_type_name
    );
}
