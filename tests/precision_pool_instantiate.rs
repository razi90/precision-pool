#[cfg(test)]
mod precision_pool_instantiate {
    use common::math::*;
    use precision_pool::{
        constants::{FLASH_LOAN_FEE_RATE_MAX, INPUT_FEE_RATE_MAX},
        pool_math::tick_to_price_sqrt,
    };
    use precision_pool_test_helper::*;
    use pretty_assertions::assert_eq;
    use scrypto::prelude::*;
    use scrypto_testenv::*;
    use std::iter::zip;
    use test_case::test_case;

    // INSTANTIATE

    #[test]
    fn test_instantiate() {
        let mut helper: PoolTestHelper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.set_whitelist_registry();
        let receipt = helper
            .instantiate(
                helper.x_address(),
                helper.y_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_success(false);
        let outputs: Vec<(ComponentAddress, ResourceAddress)> = receipt.outputs("instantiate");
        let commit_result = receipt.execution_receipt.expect_commit_success();
        assert_eq!(
            outputs,
            zip(
                commit_result.new_component_addresses().clone(),
                commit_result.new_resource_addresses().clone()
            )
            .collect::<Vec<(ComponentAddress, ResourceAddress)>>()
        );
    }

    #[test]
    fn test_instantiate_same_token() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.set_whitelist_registry();
        helper
            .instantiate(
                helper.x_address(),
                helper.x_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_instantiate_random_address_token() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        println!("{:?}", helper.a_address());
        println!("{:?}", helper.b_address());
        // random hex string with 5d as first two chars
        let random_address = ResourceAddress::try_from_hex(
            "5df173925d7814e488512f12cb03c6edfe2b3ea39c24538290476c34ba17",
        )
        .unwrap();
        helper.set_whitelist_registry();
        helper
            .instantiate(
                helper.x_address(),
                random_address,
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_rejection(false);
    }

    #[test_case(0, false ; "0")]
    #[test_case(1, true ; "1")]
    #[test_case(100, true ; "100")]
    #[test_case(MAX_TICK as u32, true ; "max_tick")]
    #[test_case((MAX_TICK as u32) + 1, false ; "above_max_tick")]
    fn test_instantiate_tick_spacing(tick_spacing: u32, execute_expect_success: bool) {
        let mut helper: PoolTestHelper = PoolTestHelper::new();
        helper.instantiate_tick_spacing(tick_spacing);
        if execute_expect_success {
            helper.registry.execute_expect_success(false);
        } else {
            helper.registry.execute_expect_failure(false);
        }
    }

    #[test]
    fn test_instantiate_price_positive() {
        PoolTestHelper::new().instantiate_default(pdec!(1), false);
    }

    #[test]
    #[should_panic]
    fn test_instantiate_price_negative() {
        PoolTestHelper::new().instantiate_default(pdec!(-1), false);
    }

    #[test]
    #[should_panic]
    fn test_instantiate_price_zero() {
        PoolTestHelper::new().instantiate_default(pdec!(0), false);
    }

    #[test]
    fn test_instantiate_price_min() {
        PoolTestHelper::new().instantiate_default(tick_to_price_sqrt(MIN_TICK), false);
    }

    #[test]
    fn test_instantiate_price_max() {
        PoolTestHelper::new().instantiate_default(tick_to_price_sqrt(MAX_TICK), false);
    }

    #[test]
    #[should_panic]
    fn test_instantiate_price_below_min_fail() {
        PoolTestHelper::new()
            .instantiate_default(tick_to_price_sqrt(MIN_TICK) - PreciseDecimal::ATTO, false);
    }

    #[test]
    #[should_panic]
    fn test_instantiate_price_above_max_fail() {
        PoolTestHelper::new()
            .instantiate_default(tick_to_price_sqrt(MAX_TICK) + PreciseDecimal::ATTO, false);
    }

    #[test]
    fn test_instantiate_nft_addresses_both() {
        let mut helper: PoolTestHelper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.set_whitelist_registry();
        helper
            .instantiate(
                helper.j_nft_address(),
                helper.k_nft_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_instantiate_nft_address_x() {
        let mut helper: PoolTestHelper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.set_whitelist_registry();
        helper
            .instantiate(
                helper.j_nft_address(),
                helper.y_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_instantiate_nft_address_y() {
        let mut helper: PoolTestHelper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.set_whitelist_registry();
        helper
            .instantiate(
                helper.x_address(),
                helper.k_nft_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_instantiate_wrong_order() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.set_whitelist_registry();
        // a_address < b_address
        helper.instantiate(
            helper.b_address(),
            helper.a_address(),
            pdec!(1),
            dec!(0),
            dec!(0),
            helper.registry.registry_address.unwrap(),
            vec![],
        );
        helper.registry.execute_expect_failure(false);
    }

    #[test]
    fn test_after_instantiate() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.set_whitelist_registry();
        helper.instantiate(
            helper.x_address(),
            helper.y_address(),
            pdec!(1),
            dec!(0),
            dec!(0),
            helper.registry.registry_address.unwrap(),
            vec![],
        );
        let receipt = helper.registry.execute_expect_success(false);
        let (pool_address, lp_address): (ComponentAddress, ResourceAddress) =
            receipt.outputs("instantiate")[0];
        helper.pool_address = Some(pool_address);
        helper.lp_address = Some(lp_address);
        helper
            .execute_after_instantiate(pdec!(1), helper.a_address(), helper.a_address())
            .registry
            .execute_expect_failure(false);
    }

    fn instantiate_with_input_fee_rate_test(input_fee_rate: Decimal, expect_success: bool) {
        let mut helper = PoolTestHelper::new();
        helper.set_whitelist_registry();
        helper.instantiate(
            helper.x_address(),
            helper.y_address(),
            pdec!(1),
            input_fee_rate,
            dec!(0),
            helper.registry.registry_address.unwrap(),
            vec![],
        );

        if expect_success {
            helper.registry.execute_expect_success(false);
        } else {
            helper.registry.execute_expect_failure(false);
        }
    }

    fn instantiate_with_flash_loan_fee_rate_test(
        flash_loan_fee_rate: Decimal,
        expect_success: bool,
    ) {
        let mut helper = PoolTestHelper::new();
        helper.set_whitelist_registry();
        helper.instantiate(
            helper.x_address(),
            helper.y_address(),
            pdec!(1),
            dec!(0),
            flash_loan_fee_rate,
            helper.registry.registry_address.unwrap(),
            vec![],
        );

        if expect_success {
            helper.registry.execute_expect_success(false);
        } else {
            helper.registry.execute_expect_failure(false);
        }
    }

    // Test input fee rates

    #[test]
    fn test_instantiate_with_input_fee_rate_zero() {
        instantiate_with_input_fee_rate_test(dec!(0), true);
    }

    #[test]
    fn test_instantiate_with_input_fee_rate_max() {
        instantiate_with_input_fee_rate_test(INPUT_FEE_RATE_MAX, true);
    }

    #[test]
    fn test_instantiate_with_input_fee_rate_mid() {
        instantiate_with_input_fee_rate_test(INPUT_FEE_RATE_MAX / 2, true);
    }

    #[test]
    fn test_instantiate_with_input_fee_rate_fail_negative() {
        instantiate_with_input_fee_rate_test(dec!(0) - Decimal::ATTO, false);
    }

    #[test]
    fn test_instantiate_with_input_fee_rate_fail_higher_than_max() {
        instantiate_with_input_fee_rate_test(INPUT_FEE_RATE_MAX + Decimal::ATTO, false);
    }

    #[test]
    fn test_instantiate_with_input_fee_rate_fail_higher_than_one() {
        instantiate_with_input_fee_rate_test(dec!(1) + Decimal::ATTO, false);
    }

    // Test flash loan fee rates

    #[test]
    fn test_instantiate_with_flash_loan_fee_rate_zero() {
        instantiate_with_flash_loan_fee_rate_test(dec!(0), true);
    }

    #[test]
    fn test_instantiate_with_flash_loan_fee_rate_max() {
        instantiate_with_flash_loan_fee_rate_test(FLASH_LOAN_FEE_RATE_MAX, true);
    }

    #[test]
    fn test_instantiate_with_flash_loan_fee_rate_mid() {
        instantiate_with_flash_loan_fee_rate_test(FLASH_LOAN_FEE_RATE_MAX / 2, true);
    }

    #[test]
    fn test_instantiate_with_flash_loan_fee_rate_fail_negative() {
        instantiate_with_flash_loan_fee_rate_test(dec!(0) - Decimal::ATTO, false);
    }

    #[test]
    fn test_instantiate_with_flash_loan_fee_rate_fail_higher_than_max() {
        instantiate_with_flash_loan_fee_rate_test(FLASH_LOAN_FEE_RATE_MAX + Decimal::ATTO, false);
    }

    #[test]
    fn test_instantiate_with_flash_loan_fee_rate_fail_higher_than_one() {
        instantiate_with_flash_loan_fee_rate_test(dec!(1) + Decimal::ATTO, false);
    }

    #[test]
    fn test_instantiate_registry_metadata_other_value_type() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.set_whitelist_registry_value("OTHER");
        helper
            .instantiate(
                helper.x_address(),
                helper.y_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_instantiate_registry_metadata_other_value_type_vec() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.set_whitelist_registry_value(vec!["FAKE"]);
        helper
            .instantiate(
                helper.x_address(),
                helper.y_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_instantiate_registry_metadata_other_package_address() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        let global_address: GlobalAddress = helper.registry.env.account.into();
        helper.set_whitelist_registry_value(vec![global_address]);
        helper
            .instantiate(
                helper.x_address(),
                helper.y_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_instantiate_registry_metadata_two_package_address_registry_and_other() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        let global_address1: GlobalAddress = helper.registry.registry_address.unwrap().into();
        let global_address2: GlobalAddress = helper.registry.env.account.into();
        helper.set_whitelist_registry_value(vec![global_address1, global_address2]);
        helper
            .instantiate(
                helper.x_address(),
                helper.y_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_success(false);
    }

    #[test]
    fn test_instantiate_registry_metadata_two_same_registry_package_addresses() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        let global_address1: GlobalAddress = helper.registry.registry_address.unwrap().into();
        let global_address2: GlobalAddress = helper.registry.registry_address.unwrap().into();
        helper.set_whitelist_registry_value(vec![global_address1, global_address2]);
        helper
            .instantiate(
                helper.x_address(),
                helper.y_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_success(false);
    }

    #[test]
    fn test_instantiate_registry_metadata_two_addresses_registry_and_resource() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        let global_address1: GlobalAddress = helper.registry.registry_address.unwrap().into();
        let global_address2: GlobalAddress = helper.registry.env.x_address.into();
        helper.set_whitelist_registry_value(vec![global_address1, global_address2]);
        helper
            .instantiate(
                helper.x_address(),
                helper.y_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_success(false);
    }

    #[test]
    fn test_instantiate_registry_metadata_empty_vec() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.set_whitelist_registry_value(Vec::<GlobalAddress>::new());
        helper
            .instantiate(
                helper.x_address(),
                helper.y_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_instantiate_registry_metadata_missing() {
        let mut helper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper
            .instantiate(
                helper.x_address(),
                helper.y_address(),
                pdec!(1),
                dec!(0),
                dec!(0),
                helper.registry.registry_address.unwrap(),
                vec![],
            )
            .registry
            .execute_expect_failure(false);
    }

    /*
    #[test]
    fn test_lp_address() {
        let mut helper = PoolTestHelper::new();
        helper.registry.instantiate_default(helper.registry.admin_badge_address());
        helper.instantiate_default(dec!(1), false);
        let receipt = helper.lp_address().execute_success(false);
        let lp_address: Vec<ResourceAddress> = receipt.outputs("lp_address");
        assert_eq!(
            lp_address,
            vec![
                ResourceAddress::try_from_hex(
                    "02c3b29eafd58f639f0fcc45cc83161f6f877e5b1541ed67ef0248"
                ).unwrap()
            ]
        );
    }*/
}
