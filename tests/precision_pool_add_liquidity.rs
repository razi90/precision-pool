#[cfg(test)]
mod precision_pool_add_liquidity {
    use common::math::*;
    use precision_pool::pool_math::tick_to_price_sqrt;
    use precision_pool_test_helper::*;
    use pretty_assertions::assert_eq;
    use radix_engine::system::system_modules::execution_trace::{
        ResourceSpecifier::Amount, ResourceSpecifier::Ids,
    };
    use scrypto::prelude::*;
    use scrypto_testenv::*;
    use test_case::test_case;

    #[test]
    fn test_add_liquidity_invalid_token_both() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1), false);
        helper
            .add_liquidity(
                -1,
                1,
                helper.u_address(),
                dec!(1),
                helper.v_address(),
                dec!(1),
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_add_liquidity_invalid_token_x() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1), false);
        helper
            .add_liquidity(
                -1,
                1,
                helper.u_address(),
                dec!(1),
                helper.y_address(),
                dec!(1),
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test]
    fn test_add_liquidity_invalid_token_y() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1), false);
        helper
            .add_liquidity(
                -1,
                1,
                helper.x_address(),
                dec!(1),
                helper.v_address(),
                dec!(1),
            )
            .registry
            .execute_expect_failure(false);
    }

    #[test_case(10, -1, 1, 0, 0, false, false ; "mapped_to_same_tick_fail")]
    #[test_case(10, -12, 13, -10, 10, true, true ; "10_mapped_lower")]
    #[test_case(10, -17, 18, -10, 10, true, true ; "10_mapped_lower_2")]
    #[test_case(10, -19, 19, -10, 10, true, true ; "10_mapped_lower_3")]
    #[test_case(10, -21, 18, -20, 10, true, true ; "10_mapped_lower_4")]
    #[test_case(10, -21, 18, -20, 10, true, true ; "10_mapped_lower_5")]
    #[test_case(10, -10, 10, -10, 10, true, true ; "10_matched")]
    #[test_case(10, -10, 8, -10, 0, true, true ; "10_matched_crossed_1")]
    #[test_case(10, -8, 10, 0, 10, true, true ; "10_matched_crossed_2")]
    #[test_case(10, -20, 20, -20, 20, true, true ; "10_multiple_matched")]
    #[test_case(10, -20, 18, -20, 10, true, true ; "10_multiple_matched_crossed_1")]
    #[test_case(10, -18, 20, -10, 20, true, true ; "10_multiple_matched_crossed_2")]
    #[test_case(10, MIN_TICK, MAX_TICK, -887270, 887270, true, true ; "10_min_max_tick")]
    #[test_case(MAX_TICK as u32, MIN_TICK+1, 5000, 0, 0, false, false ; "max_tick_left_match_right_below")]
    #[test_case(MAX_TICK as u32, MIN_TICK, MAX_TICK, MIN_TICK, MAX_TICK, true, true ; "max_tick_both_match")]
    #[test_case(10, MIN_TICK-2, MAX_TICK+3, -887270, 887270, false, false ; "ticks_outside_range")]
    #[test_case(10, MIN_TICK-40, MAX_TICK+40, -887270, 887270, false, false ; "ticks_outside_range_2")]
    fn test_add_liquidity_tick_spacing(
        tick_spacing: u32,
        left_bound: i32,
        right_bound: i32,
        left_bound_expected: i32,
        right_bound_expected: i32,
        update_existing_tick: bool,
        expect_execution_success: bool,
    ) {
        add_liquidity_tick_spacing_assert_ticks(
            tick_spacing,
            left_bound,
            right_bound,
            left_bound_expected,
            right_bound_expected,
            false, // insert new tick
            expect_execution_success,
        );
        if update_existing_tick {
            add_liquidity_tick_spacing_assert_ticks(
                tick_spacing,
                left_bound,
                right_bound,
                left_bound_expected,
                right_bound_expected,
                true, // update existing tick
                expect_execution_success,
            );
        }
    }

    #[test_case(1, MIN_TICK, MIN_TICK + 1, dec!(1), false ; "spacing_1_min_tick_fail")]
    #[test_case(1, MIN_TICK, MIN_TICK + 1, dec!(0.000000001), true ; "spacing_1_min_tick_pass")]
    #[test_case(1, 0, 1, dec!(88400000000), true    ; "spacing_1_pass")]
    #[test_case(1, 0, 1, dec!(88500000000), false   ; "spacing_1_fail")]
    #[test_case(2, 0, 2, dec!(353707935005), true    ; "spacing_2_pass")]
    #[test_case(2, 0, 2, dec!(353707935006), false   ; "spacing_2_fail")]
    #[test_case(MAX_TICK, 0, MAX_TICK, dec!(1046222947221001933287), true   ; "spacing_max_tick_pass")]
    #[test_case(MAX_TICK, 0, MAX_TICK, dec!(1046222947221001933288), false   ; "spacing_max_tick_fail")]
    fn test_add_liquidity_assert_maximum_liquidity(
        tick_spacing: i32,
        left_bound: i32,
        right_bound: i32,
        amount: Decimal,
        expect_execution_success: bool,
    ) {
        add_liquidity_tick_spacing_assert_amount(
            tick_spacing as u32,
            left_bound,
            right_bound,
            amount,
            amount,
            false, // insert new tick
            expect_execution_success,
        );
        add_liquidity_tick_spacing_assert_amount(
            tick_spacing as u32,
            left_bound,
            right_bound,
            amount,
            amount,
            true, // update existing tick
            expect_execution_success,
        );
    }

    #[test]
    fn test_add_liquidity_bounds_equal() {
        add_liquidity_bounds_expect_failure(pdec!(1), 1, 1);
    }

    #[test]
    fn test_add_liquidity_bounds_left_larger_right() {
        add_liquidity_bounds_expect_failure(pdec!(1), 1, -1);
    }

    #[test]
    fn test_add_liquidity_bounds_minimum() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            MIN_TICK,
            2,
            dec!(1),
            dec!(1),
            dec!("0.999950008748812645"),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_bounds_minimum_less_than_minimum() {
        add_liquidity_bounds_expect_failure(pdec!(1), MIN_TICK - 1, 1);
    }

    #[test]
    fn test_add_liquidity_bounds_maximum() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            0,
            MAX_TICK,
            dec!(1),
            dec!(1),
            dec!("0"),
            dec!("0.999949998750062495"),
        );
    }

    #[test]
    fn test_add_liquidity_bounds_left_equal_zero() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            0,
            2,
            dec!(1),
            dec!(1),
            dec!("0.000099990000999901"),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_bounds_right_equal_zero() {
        add_liquidity_expect_success(
            tick_to_price_sqrt(-1),
            -2,
            0,
            dec!(1),
            dec!(1),
            dec!(0),
            dec!("0.000099990000999901"),
        )
    }

    #[test]
    fn test_add_liquidity_minimum_range_price_lesser_left_bound() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            2,
            3,
            dec!(1),
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_add_liquidity_minimum_range_price_equal_left_bound() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            1,
            2,
            dec!(1),
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_add_liquidity_minimum_range_price_between_bounds() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            0,
            2,
            dec!(1),
            dec!(1),
            dec!("0.000099990000999901"),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_minimum_range_price_equal_right_bound() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            0,
            1,
            dec!(1),
            dec!(1),
            dec!(1),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_minimum_range_price_greater_right_bound() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            -1,
            0,
            dec!(1),
            dec!(1),
            dec!(1),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_large_range_price_lesser_left_bound() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            100,
            100000,
            dec!(1),
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_add_liquidity_large_range_price_equal_left_bound() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            1,
            100000,
            dec!(1),
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_add_liquidity_large_range_price_between_bounds() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            -100000,
            100000,
            dec!(1),
            dec!(1),
            dec!("0.502394229632486608"),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_large_range_price_equal_right_bound() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            -100000,
            1,
            dec!(1),
            dec!(1),
            dec!(1),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_large_range_price_greater_right_bound() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            -100000,
            -100,
            dec!(1),
            dec!(1),
            dec!(1),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_medium_range_price_between_bounds() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            -10000,
            10000,
            dec!(1),
            dec!(1),
            dec!("0.875492168348941143"),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_symmetric() {
        add_liquidity_expect_success(pdec!(1), -10000, 10000, dec!(1), dec!(1), dec!(0), dec!(0));
    }

    #[test]
    fn test_add_liquidity_zero_amount() {
        add_liquidity_expect_failure(*PRICE_BETWEEN_LEFT_BOUNDS_SQRT, 0, 2, dec!(0), dec!(0));
    }

    #[test]
    fn test_add_liquidity_zero_liquidity_price_left_of_range() {
        add_liquidity_expect_failure(*PRICE_BETWEEN_LEFT_BOUNDS_SQRT, 2, 3, dec!(0), dec!(1));
    }

    #[test]
    fn test_add_liquidity_zero_liquidity_price_equal_left_bound() {
        add_liquidity_expect_failure(*PRICE_BETWEEN_LEFT_BOUNDS_SQRT, 1, 2, dec!(0), dec!(1));
    }

    #[test]
    fn test_add_liquidity_zero_liquidity_price_equal_right_bound() {
        add_liquidity_expect_failure(*PRICE_BETWEEN_LEFT_BOUNDS_SQRT, 0, 1, dec!(1), dec!(0));
    }

    #[test]
    fn test_add_liquidity_zero_liquidity_price_right_of_range() {
        add_liquidity_expect_failure(*PRICE_BETWEEN_LEFT_BOUNDS_SQRT, -1, 0, dec!(1), dec!(0));
    }

    #[test]
    fn test_add_liquidity_zero_liquidity_amount_too_low() {
        add_liquidity_expect_failure(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            0,
            2,
            dec!("0.000000000000000002"),
            dec!("0.000000000000000002"),
        );
    }

    #[test]
    fn test_add_liquidity_minimum_range_minimum_amount() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            0,
            2,
            dec!("0.000000000000000003"),
            dec!("0.000000000000000003"),
            dec!(0),
            dec!(0),
        );
    }

    #[test]
    #[ignore]
    fn test_add_liquidity_minimum_range_maximum_amount() {
        println!("{}", MAX_SUPPLY);
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            0,
            2,
            MAX_SUPPLY,
            MAX_SUPPLY,
            dec!("570841992883095642859028.485045464760792252"), // 570841992883095642859028.484906187248463342
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_maximum_range_minimum_amount() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            MIN_TICK,
            MAX_TICK,
            dec!("0.000000000000000003"),
            dec!("0.000000000000000003"),
            dec!(0),
            dec!(0),
        );
    }

    #[test]
    #[ignore]
    fn test_add_liquidity_maximum_range_maximum_amount() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            MIN_TICK,
            MAX_TICK,
            MAX_SUPPLY,
            MAX_SUPPLY,
            dec!("570841992883095642889973.507357211993738685"), // 570841992883095642889973.507357205803699122,
            dec!(0),
        );
    }

    #[test]
    #[ignore]
    // TODO: `(left != right)`\n  left: `0`,\n right: `0`: [Add liquidity]: Allowed liquidity is zero.
    fn test_add_liquidity_minimum_possible_liquidity() {
        add_liquidity_expect_success(
            tick_to_price_sqrt(MIN_TICK),
            MIN_TICK,
            MAX_TICK,
            dec!("0.000000000000000003"),
            dec!("0.000000000000000003"),
            dec!("0"),
            dec!("0.000000000000000003"),
        );
    }

    #[test]
    #[ignore]
    // TODO: Overflow or division by zero
    fn test_add_liquidity_maximum_possible_liquidity() {
        add_liquidity_expect_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            MAX_TICK - 1,
            MAX_TICK,
            MAX_SUPPLY,
            MAX_SUPPLY,
            dec!(0),
            MAX_SUPPLY,
        );
    }

    #[test]
    fn test_add_liquidity_maximum_price() {
        add_liquidity_expect_success(
            tick_to_price_sqrt(MAX_TICK),
            0,
            2,
            dec!(1),
            dec!(1),
            dec!(1),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_minimum_price() {
        add_liquidity_expect_success(
            tick_to_price_sqrt(MIN_TICK),
            0,
            2,
            dec!(1),
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_add_liquidity_symmetric_bounds() {
        add_liquidity_expect_success(
            pdec!("2.11692063589245341590374438031104661"), // 15000
            10000,
            20000,
            dec!(1),
            dec!(4.481352978667309329),
            dec!(0),
            dec!(0) + Decimal::ATTO * 7,
        );
    }

    #[test]
    fn test_add_liquidity_symmetric_bounds_2() {
        add_liquidity_expect_success(
            pdec!("2.11692063589245341590374438031104661"), // 15000
            10000,
            20000,
            dec!(2),
            dec!(4.481352978667309329) * 2,
            dec!(0),
            dec!(0) + Decimal::ATTO * 6,
        );
    }

    #[test]
    fn test_add_liquidity_symmetric_bounds_3() {
        add_liquidity_expect_success(
            pdec!("2.11692063589245341590374438031104661"), // 15000
            10000,
            20000,
            dec!(100),
            dec!(4.481352978667309329) * 100,
            dec!(0.000000000000000012),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_symmetric_bounds_4() {
        add_liquidity_expect_success(
            pdec!("2.11692063589245341590374438031104661"), // 15000
            10000,
            20000,
            dec!(1000000),
            dec!(4.481352978667309329) * 1000000,
            dec!(0.000000000000134383),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_symmetric_bounds_multiplier_same_ratio() {
        add_liquidity_expect_success(
            pdec!("2.11692063589245341590374438031104661"), // 15000
            -10000,                                         // 10000 - 20000
            40000,                                          // 20000 + 20000
            dec!(100),
            dec!(4.481352978667309329) * 100,
            dec!(0.000000000000000012),
            dec!(0),
        );
    }

    #[test]
    fn test_add_liquidity_multiple_positions() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(*PRICE_BETWEEN_LEFT_BOUNDS_SQRT, false);
        helper.add_liquidity_default(0, 2, dec!(1), dec!(1));
        helper.add_liquidity_default(0, 2, dec!(1), dec!(1));

        let receipt = helper.registry.execute_expect_success(false);
        let output_buckets = receipt.output_buckets("add_liquidity");

        assert_eq!(
            output_buckets,
            vec![
                vec![
                    Ids(helper.lp_address.unwrap(), nft_ids!(1)),
                    Amount(helper.x_address(), dec!("0.000099990000999901")),
                    Amount(helper.y_address(), dec!(0))
                ],
                vec![
                    Ids(helper.lp_address.unwrap(), nft_ids!(2)),
                    Amount(helper.x_address(), dec!("0.000099990000999901")),
                    Amount(helper.y_address(), dec!(0))
                ]
            ]
        );
    }

    #[test]
    pub fn test_add_liquidity_position_added_at() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1), false);
        helper.advance_timestamp_by_seconds(42);
        helper.add_liquidity_success(-10, 10, dec!(10), dec!(10), dec!(0), dec!(0));

        let position: precision_pool::pool::LiquidityPosition = helper
            .registry
            .env
            .test_runner
            .get_non_fungible_data(helper.lp_address.unwrap(), nft_id!(1));

        assert_eq!(position.added_at, 42);
    }

    #[test]
    pub fn test_add_liquidity_shape() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1), false);
        let receipt = helper
            .add_liquidity_shape(
                -10,
                10,
                helper.x_address(),
                dec!(10),
                helper.y_address(),
                dec!(10),
                None,
            )
            .registry
            .execute_expect_success(false);
        let output_buckets = receipt.output_buckets("add_liquidity_shape");

        assert_eq!(
            output_buckets,
            vec![vec![
                Ids(helper.lp_address.unwrap(), nft_ids!(1, 2)),
                Amount(helper.x_address(), dec!(0)),
                Amount(helper.y_address(), dec!(0))
            ]]
        );
    }

    #[test]
    pub fn test_add_liquidity_shape_proof() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1), false);
        // add two positions
        let receipt = helper
            .add_liquidity_shape(
                -10,
                10,
                helper.x_address(),
                dec!(10),
                helper.y_address(),
                dec!(10),
                None,
            )
            .registry
            .execute_expect_success(true);
        // add too same shape_id
        let receipt = helper
            .add_liquidity_shape(
                -10,
                10,
                helper.x_address(),
                dec!(10),
                helper.y_address(),
                dec!(10),
                None,
            )
            .registry
            .execute_expect_success(true);
        let output_buckets = receipt.output_buckets("add_liquidity_shape");

        assert_eq!(
            output_buckets,
            vec![vec![
                Ids(helper.lp_address.unwrap(), nft_ids!(3, 4)),
                Amount(helper.x_address(), dec!(0)),
                Amount(helper.y_address(), dec!(0))
            ]]
        );
    }
}
