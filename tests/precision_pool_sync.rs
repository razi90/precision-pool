#[cfg(test)]
mod precision_pool_sync {
    use common::pools::SwapType;
    use precision_pool_test_helper::*;
    use pretty_assertions::assert_eq;
    use radix_engine::system::system_modules::execution_trace::ResourceSpecifier::Amount;
    use scrypto::prelude::*;
    use scrypto_testenv::*;

    #[test]
    fn test_sync_registry_config() {
        let fee_protocol_share = dec!("0.0067");
        let sync_period: u64 = 3041;
        let sync_slots: u64 = 32;

        let mut helper = PoolTestHelper::new_without_instantiate_registry();
        helper.registry.instantiate_execute(
            helper.registry.admin_badge_address(),
            fee_protocol_share,
            sync_period,
            sync_slots,
        );
        // Instantiate pool
        helper.instantiate_default(pdec!(1), false);

        helper.registry.sync(
            helper.pool_address.unwrap(),
            helper.x_address(),
            dec!("1.23"),
            helper.y_address(),
            dec!("1.24"),
        );
        let receipt = helper.registry.execute_expect_success(false);
        let output: Vec<(Decimal, u64)> = receipt.outputs("sync");

        assert_eq!(output, vec![(fee_protocol_share, 5891)]);
    }

    #[test]
    fn test_sync_update_config() {
        let mut helper = PoolTestHelper::new_without_instantiate_registry();
        helper.registry.instantiate_execute(
            helper.registry.admin_badge_address(),
            dec!("0.1"),
            1,
            1,
        );
        helper.instantiate_default(pdec!(1), false);
        helper.registry.load_owner_auth();
        helper.registry.update_config(dec!(0.2), 100, 42);
        helper.registry.sync(
            helper.pool_address.unwrap(),
            helper.x_address(),
            dec!("1"),
            helper.y_address(),
            dec!("1"),
        );
        let receipt = helper.registry.execute_expect_success(false);
        let output: Vec<(Decimal, u64)> = receipt.outputs("sync");

        assert_eq!(output, vec![(dec!(0.2), 156)]);
    }

    #[test]
    fn test_withdraw_protocol_fees_single_token() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1), false);
        helper.registry.sync(
            helper.pool_address.unwrap(),
            helper.x_address(),
            dec!("1"),
            helper.y_address(),
            dec!("2"),
        );

        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees(vec![helper.x_address()]);

        let receipt = helper.registry.execute_expect_success(false);
        let output_buckets = receipt.output_buckets("withdraw_protocol_fees");

        assert_eq!(
            output_buckets,
            vec![vec![Amount(helper.x_address(), dec!(1))]]
        );
    }

    #[test]
    fn test_withdraw_protocol_fees_single_pool() {
        let mut helper = PoolTestHelper::new_without_instantiate_registry();
        helper.registry.instantiate_execute(
            helper.registry.admin_badge_address(),
            dec!("0.1"),
            1,
            1,
        );

        helper.instantiate_default(pdec!(1), false);
        helper.registry.sync(
            helper.pool_address.unwrap(),
            helper.x_address(),
            dec!("1"),
            helper.y_address(),
            dec!("2"),
        );

        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees(vec![helper.x_address(), helper.y_address()]);

        let receipt = helper.registry.execute_expect_success(false);
        let output_buckets = receipt.output_buckets("withdraw_protocol_fees");

        assert_eq!(
            output_buckets,
            vec![vec![
                Amount(helper.x_address(), dec!(1)),
                Amount(helper.y_address(), dec!(2))
            ]]
        );
    }

    #[test]
    fn test_withdraw_protocol_fees_multiple_pool() {
        let mut helper = PoolTestHelper::new_without_instantiate_registry();
        helper.registry.instantiate_execute(
            helper.registry.admin_badge_address(),
            dec!("0.1"),
            1,
            1,
        );

        helper.instantiate_default(pdec!(1), false);
        helper.registry.sync(
            helper.pool_address.unwrap(),
            helper.x_address(),
            dec!("1"),
            helper.y_address(),
            dec!("2"),
        );
        helper.instantiate_default(pdec!(1), false);
        helper.registry.sync(
            helper.pool_address.unwrap(),
            helper.x_address(),
            dec!("3"),
            helper.y_address(),
            dec!("4"),
        );

        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees(vec![helper.x_address(), helper.y_address()]);

        let receipt = helper.registry.execute_expect_success(false);
        let output_buckets = receipt.output_buckets("withdraw_protocol_fees");

        assert_eq!(
            output_buckets,
            vec![vec![
                Amount(helper.x_address(), dec!(4)),
                Amount(helper.y_address(), dec!(6))
            ]]
        );
    }

    #[test]
    fn test_sync_pool_swap_advance_time() {
        let fee_protocol_share = dec!(0.25);
        let sync_period: u64 = 3041;
        let sync_slots: u64 = 32;

        let mut helper = PoolTestHelper::new_without_instantiate_registry();
        helper.registry.instantiate_execute(
            helper.registry.admin_badge_address(),
            fee_protocol_share,
            sync_period,
            sync_slots,
        );

        // Instantiate pool
        helper.instantiate_default_with_input_fee(pdec!(1), dec!(0.1), false);
        helper.add_liquidity_success(-10000, 10000, dec!(10), dec!(10), dec!(0), dec!(0));

        helper.swap_success(SwapType::BuyX, dec!(3), dec!(2.440716293858554572), dec!(0));
        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees_success(dec!(0), dec!(0));

        helper.swap_success(SwapType::BuyX, dec!(2), dec!(1.382386594462737807), dec!(0));
        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees_success(dec!(0), dec!(0));

        helper.swap_success(
            SwapType::SellX,
            dec!(1),
            dec!(1.197018932606031926),
            dec!(0),
        );
        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees_success(dec!(0), dec!(0));

        helper.advance_timestamp_by_seconds(5891);

        helper.swap_success(SwapType::BuyX, dec!(4), dec!(2.505473242730227614), dec!(0));
        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees_success(dec!(0.025), dec!(0.125));

        helper.advance_timestamp_by_seconds(sync_period);

        helper.swap_success(SwapType::BuyX, dec!(1), dec!(0.541517298529914214), dec!(0));
        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees_success(dec!(0), dec!(0.1));

        helper.advance_timestamp_by_seconds(sync_period + 1);

        helper.swap_success(SwapType::BuyX, dec!(1), dec!(0.512948643029352728), dec!(0));
        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees_success(dec!(0), dec!(0.025));

        helper.advance_timestamp_by_seconds(sync_period); // still below next sync time due to ceiling next slot time

        helper.swap_success(SwapType::BuyX, dec!(1), dec!(0.486582665710678144), dec!(0));
        helper.swap_success(
            SwapType::SellX,
            dec!(3),
            dec!(4.471281200626990123),
            dec!(0),
        );
        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees_success(dec!(0), dec!(0));

        helper.advance_timestamp_by_seconds(sync_period); // we now cross the next sync time and can collect protocol fees
        helper.swap_success(SwapType::BuyX, dec!(1), dec!(0.605186207582635607), dec!(0));
        helper.registry.load_owner_auth();
        helper
            .registry
            .withdraw_protocol_fees_success(dec!(0.075), dec!(0.05));
    }
}
