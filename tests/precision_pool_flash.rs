#[cfg(test)]
mod precision_pool_flash {
    use scrypto::prelude::*;
    use scrypto_testenv::*;
    use precision_pool_test_helper::*;

    static ONE_LP: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: TICK_LEFT_BOUND,
        right_bound: TICK_RIGHT_BOUND,
        x_amount: DEC_10,
        y_amount: DEC_10,
    }];

    #[test]
    fn test_take_loan_only() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_flash_loan_fee(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            dec!(0.009),
            true,
        );
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper
            .flash_loan(helper.x_address(), dec!(1))
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_take_too_much_loan() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_flash_loan_fee(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            dec!(0.009),
            true,
        );
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper
            .flash_loan(helper.x_address(), dec!(20))
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_take_repay_loan() {
        let mut helper = PoolTestHelper::new();
        helper.repay_loan_success(helper.x_address(), dec!(1), dec!(1), dec!("0.009"), dec!(0));
    }

    #[test]
    fn test_take_loan_repay_insufficient() {
        let mut helper = PoolTestHelper::new();
        helper.repay_loan_failure(helper.x_address(), dec!(1), dec!(0));
    }

    #[test]
    fn test_take_loan_repay_more() {
        let mut helper = PoolTestHelper::new();
        helper.repay_loan_success(helper.x_address(), dec!(1), dec!(1), dec!("1.009"), dec!(1));
    }

    #[test]
    fn test_take_loan_repay_wrong_token() {
        let mut helper = PoolTestHelper::new();
        helper.repay_loan_failure(helper.y_address(), dec!("1.009"), dec!(0));
    }

    #[test]
    fn test_take_two_loans_one_repay() {
        let mut helper = PoolTestHelper::new();

        helper.instantiate_default_with_flash_loan_fee(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            dec!(0.009),
            true,
        );
        let receipt = helper
            .add_liquidity_default_batch(&ONE_LP)
            .flash_loan_address()
            .registry
            .execute_expect_success(false);
        let transient_address: ResourceAddress = receipt.outputs("flash_loan_address")[0];

        helper.flash_loan(helper.x_address(), dec!(1));
        helper.flash_loan(helper.x_address(), dec!(1));
        helper
            .repay_loan(
                helper.x_address(),
                dec!("1"),
                dec!("0.009"),
                transient_address,
                dec!(2),
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_take_repay_loan_divisibility() {
        let mut helper = PoolTestHelper::new();
        let other_address = helper.registry.env.test_runner.create_fungible_resource(
            dec!(10000000),
            15,
            helper.registry.env.account,
        );
        let stable_address = helper.registry.env.test_runner.create_fungible_resource(
            dec!(10000000),
            6,
            helper.registry.env.account,
        );
        helper.registry.env.x_address = stable_address;
        helper.registry.env.y_address = other_address;

        helper.repay_loan_success(
            helper.x_address(),
            dec!(1.39249343434345),
            dec!(1.392493),
            dec!("0.012533"),
            dec!(0),
        );
    }
}
