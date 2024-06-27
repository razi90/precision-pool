use precision_pool_test_helper::*;
use pretty_assertions::assert_eq;
use radix_engine::system::system_modules::execution_trace::ResourceSpecifier;
use scrypto::prelude::*;
use scrypto_testenv::environment::TestHelperExecution;
use scrypto_testenv::nft_ids;
use test_case::test_case;

#[test_case(1)]
#[test_case(2)]
#[test_case(3)]
fn test_tick_spacing(tick_spacing: u32) {
    let mut helper = PoolTestHelper::new();
    let receipt = helper
        .instantiate_tick_spacing(tick_spacing)
        .registry
        .execute_expect_success(false);

    let (pool_address, lp_address): (ComponentAddress, ResourceAddress) =
        receipt.outputs("instantiate")[0];
    helper.pool_address = Some(pool_address);
    helper.lp_address = Some(lp_address);

    let receipt = helper.tick_spacing().registry.execute_expect_success(false);
    let result: Vec<u32> = receipt.outputs("tick_spacing");

    assert_eq!(result, vec![tick_spacing]);
}

#[test]
fn test_getters() {
    let price_sqrt = pdec!(23.18726);
    let input_fee_rate = dec!(0.003251);
    let flash_loan_fee_rate = dec!(0.01021);

    let mut helper = PoolTestHelper::new();
    helper
        .instantiate_default_with_fees_and_hooks(
            price_sqrt,
            input_fee_rate,
            flash_loan_fee_rate,
            vec![],
            false,
        )
        .registry
        .execute_expect_success(false);

    helper.price_sqrt();
    helper.active_liquidity();
    helper.input_fee_rate();
    helper.flash_loan_fee_rate();

    let receipt = helper.registry.execute_expect_success(false);

    let price_sqrt_returned: Vec<PreciseDecimal> = receipt.outputs("price_sqrt");
    let input_fee_rate_returned: Vec<Decimal> = receipt.outputs("input_fee_rate");
    let flash_loan_fee_rate_returned: Vec<Decimal> = receipt.outputs("flash_loan_fee_rate");

    assert_eq!(
        (
            price_sqrt_returned,
            input_fee_rate_returned,
            flash_loan_fee_rate_returned
        ),
        (
            vec![price_sqrt],
            vec![input_fee_rate],
            vec![flash_loan_fee_rate]
        )
    );
}

#[test]
fn test_active_liquidity() {
    let active_liquidity_expected = pdec!(405.040831741441326012844520303631241918);

    let mut helper = PoolTestHelper::new();
    helper
        .instantiate_default(pdec!(1), false)
        .registry
        .execute_expect_success(false);

    helper.add_liquidity_default(-500, 500, dec!(10), dec!(10));

    helper.active_liquidity();

    let receipt = helper.registry.execute_expect_success(false);

    let active_liquidity_returned: Vec<PreciseDecimal> = receipt.outputs("active_liquidity");

    assert_eq!(active_liquidity_returned, vec![active_liquidity_expected]);
}

#[test]
fn test_lp_address() {
    let mut helper = PoolTestHelper::new();
    helper
        .registry
        .instantiate_default(helper.registry.admin_badge_address());
    helper.instantiate_default(pdec!(1), false);

    helper.add_liquidity_default(-500, 500, dec!(10), dec!(10));
    helper.lp_address();

    let receipt = helper.registry.execute_expect_success(false);

    let output_buckets = receipt.output_buckets("add_liquidity");
    let lp_address: ResourceAddress = receipt.outputs("lp_address")[0];

    assert_eq!(
        ResourceSpecifier::Ids(lp_address, nft_ids!(1)),
        output_buckets[0][0]
    );
}
