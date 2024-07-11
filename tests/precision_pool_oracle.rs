#[cfg(test)]
mod precision_pool_oracle {
    use common::pools::SwapType;
    use precision_pool_test_helper::*;
    use scrypto::prelude::*;
    use scrypto_testenv::TestHelperExecution;

    #[test]
    fn test_oracle_last_observation_index() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1.1), false);
        helper.add_liquidity_success(
            -10000,
            10000,
            dec!(10000),
            dec!(15000),
            dec!(803.246769859789666171),
            dec!(0),
        );
        helper.jump_to_timestamp_minutes(15);
        helper.swap_success(
            SwapType::SellX,
            Decimal::ONE,
            dec!("1.20995621575035767"),
            dec!(0),
        );
        helper.jump_to_timestamp_minutes(30);
        helper.swap_success(
            SwapType::BuyX,
            Decimal::ONE,
            dec!("0.826481376795724754"),
            dec!(0),
        );
        helper.jump_to_timestamp_minutes(45);
        helper.swap_success(
            SwapType::SellX,
            Decimal::ONE,
            dec!("1.209941021402946331"),
            dec!(0),
        );
        helper.jump_to_timestamp_minutes(60);

        let receipt = helper
            .last_observation_index()
            .registry
            .execute_expect_success(false);
        let outputs: Vec<Option<u16>> = receipt.outputs("last_observation_index");

        assert_eq!(outputs, vec![Some(1)]);

        let receipt = helper
            .oldest_observation_at()
            .registry
            .execute_expect_success(false);
        let outputs: Vec<Option<u64>> = receipt.outputs("oldest_observation_at");

        assert_eq!(outputs, vec![Some(1800)]);
    }
}
