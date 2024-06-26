#[cfg(test)]
mod precision_pool_swap {
    use common::math::*;
    use common::pools::SwapType;
    use precision_pool::constants::INPUT_FEE_RATE_MAX;
    use precision_pool::pool_math::tick_to_price_sqrt;
    use pretty_assertions::assert_eq;
    use scrypto::prelude::*;
    use scrypto_testenv::*;
    use test_case::test_case;
    use precision_pool_test_helper::*;

    static NO_LP: [LiquidityPosition; 0] = [];

    #[test]
    fn test_swap_no_lp_initially_buy_x() {
        swap_buy_success(pdec!("1.0001"), &NO_LP, dec!(1), dec!(0), dec!(1));
    }

    #[test]
    fn test_swap_no_lp_initially_sell_x() {
        swap_sell_success(pdec!("1.0001"), &NO_LP, dec!(1), dec!(0), dec!(1));
    }

    /*
    TEST POLICY:
    Cross as many ticks as possible while ending with a partial swap, but cross at least one tick.
    */

    static ONE_LP: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: TICK_LEFT_BOUND,
        right_bound: TICK_RIGHT_BOUND,
        x_amount: DEC_10,
        y_amount: DEC_10,
    }];

    // Test hook input_fee_rate in bounds
    #[test_case(Some(dec!(0)), None, true ; "1")]
    #[test_case(Some(INPUT_FEE_RATE_MAX / 2), None, true ; "2")]
    #[test_case(Some(INPUT_FEE_RATE_MAX), None, true ; "3")]
    #[test_case(Some(-Decimal::ATTO), None, false ; "4")]
    #[test_case(Some(INPUT_FEE_RATE_MAX + Decimal::ATTO), None, false ; "5")]
    #[test_case(None, Some(dec!(0)), true ; "6")]
    #[test_case(None, Some(INPUT_FEE_RATE_MAX / 2), true ; "7")]
    #[test_case(None, Some(INPUT_FEE_RATE_MAX), true ; "8")]
    #[test_case(None, Some(-Decimal::ATTO), false ; "9")]
    #[test_case(None, Some(INPUT_FEE_RATE_MAX + Decimal::ATTO), false ; "10")]
    fn test_swap_hook_provided_input_fee_rate(
        before_swap_rate: Option<Decimal>,
        after_swap_rate: Option<Decimal>,
        expect_success: bool,
    ) {
        swap_with_hook_action_test(
            "set_input_fee_rates",
            before_swap_rate,
            after_swap_rate,
            expect_success,
        );
    }

    // Test hook bucket amount in bounds
    #[test_case(Some(dec!(0.8)), None, false; "1")]
    #[test_case(Some(dec!(0.89)), None, false; "2")]
    #[test_case(Some(dec!(0.9)), None, true; "3")]
    #[test_case(Some(dec!(1)), None, true; "4")]
    #[test_case(None, Some(dec!(0.8)), false; "5")]
    #[test_case(None, Some(dec!(0.89)), false; "6")]
    #[test_case(None, Some(dec!(0.9)), true; "7")]
    #[test_case(None, Some(dec!(1)), true; "8")]
    #[test_case(Some(dec!(1.01)), None, false; "9")]
    #[test_case(None, Some(dec!(1.01)), false; "10")]
    fn test_swap_hook_returned_buckets(
        before_swap_rate: Option<Decimal>,
        after_swap_rate: Option<Decimal>,
        expect_success: bool,
    ) {
        swap_with_hook_action_test(
            "set_bucket_returned_fractions",
            before_swap_rate,
            after_swap_rate,
            expect_success,
        );
    }

    fn swap_with_input_fee_rate_success(
        input_fee_rate: Decimal,
        swap_type: SwapType,
        input_amount: Decimal,
        output_expected: Decimal,
        remainder_expected: Decimal,
    ) {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_input_fee(pdec!(1.4), input_fee_rate, false);
        helper.add_liquidity(
            TICK_LEFT_BOUND,
            TICK_RIGHT_BOUND,
            helper.x_address(),
            DEC_10,
            helper.y_address(),
            DEC_10,
        );
        helper.swap_success(swap_type, input_amount, output_expected, remainder_expected);
    }

    #[test]
    fn test_swap_provided_input_fee_rate_max() {
        swap_with_input_fee_rate_success(
            INPUT_FEE_RATE_MAX,
            SwapType::BuyX,
            dec!(1),
            dec!(0.436898458031114271),
            dec!(0),
        );
        swap_with_input_fee_rate_success(
            INPUT_FEE_RATE_MAX,
            SwapType::SellX,
            dec!(1),
            dec!(1.603672480661415095),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_provided_input_fee_rate_mid() {
        swap_with_input_fee_rate_success(
            INPUT_FEE_RATE_MAX / 2,
            SwapType::BuyX,
            dec!(1),
            dec!(0.459930513407366236),
            dec!(0),
        );
        swap_with_input_fee_rate_success(
            INPUT_FEE_RATE_MAX / 2,
            SwapType::SellX,
            dec!(1),
            dec!(1.684260943651846299),
            dec!(0),
        );
    }

    // SWAP ONE LP - SWAP NO TICK CROSSING - BUY/SELL
    #[test]
    fn test_swap_one_lp_not_crossing_tick_buy_x() {
        swap_buy_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &ONE_LP,
            dec!(1),
            dec!("0.472987345701045226"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_one_lp_not_crossing_tick_sell_x() {
        swap_sell_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &ONE_LP,
            dec!(1),
            dec!("1.794975288895955088"),
            dec!(0),
        );
    }

    // SWAP ONE LP - CONSUME FULL LIQUIDITY
    #[test]
    fn test_swap_one_lp_full_liquidity_price_lesser_left_bound_buy() {
        swap_buy_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &ONE_LP,
            dec!(13),
            dec!("9.999999999999999998") - Decimal::ATTO,
            dec!("0.159906324597254835"),
        );
    }

    #[test]
    fn test_swap_one_lp_full_liquidity_price_greater_right_bound_sell() {
        swap_sell_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &ONE_LP,
            dec!(13),
            dec!("9.999999999999999998") - Decimal::ATTO,
            dec!("5.211894825069227677"),
        );
    }

    // END WITH PARTIAL SWAP as DEFAULT
    #[test]
    fn test_swap_one_lp_price_lesser_left_bound_buy() {
        swap_buy_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &ONE_LP,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_one_lp_price_lesser_left_bound_sell() {
        swap_sell_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &ONE_LP,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_one_lp_price_equal_left_bound_buy() {
        let mut helper =
            PoolTestHelper::new_default_with_positions(*PRICE_LEFT_BOUND_SQRT, &ONE_LP);
        helper.swap_success(
            SwapType::BuyX,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_one_lp_price_equal_left_bound_sell_buy() {
        let mut helper =
            PoolTestHelper::new_default_with_positions(*PRICE_LEFT_BOUND_SQRT, &ONE_LP);
        helper.swap_success(SwapType::SellX, dec!(1), dec!(0), dec!(1));
        helper.swap_success(
            SwapType::BuyX,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_one_lp_price_equal_left_bound_sell_sell_buy() {
        let mut helper =
            PoolTestHelper::new_default_with_positions(*PRICE_LEFT_BOUND_SQRT, &ONE_LP);
        helper.swap_success(SwapType::SellX, dec!(1), dec!(0), dec!(1));
        helper.swap_success(SwapType::SellX, dec!(1), dec!(0), dec!(1));
        helper.swap_success(
            SwapType::BuyX,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_one_lp_price_between_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &ONE_LP,
            dec!(1),
            dec!("0.472987345701045226"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_one_lp_price_between_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &ONE_LP,
            dec!(1),
            dec!("1.794975288895955088"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_one_lp_price_equal_right_bound_sell() {
        let mut helper =
            PoolTestHelper::new_default_with_positions(*PRICE_RIGHT_BOUND_SQRT, &ONE_LP);
        helper.swap_success(
            SwapType::SellX,
            dec!(1),
            dec!("3.395647723295588275"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_one_lp_price_equal_right_bound_buy_sell() {
        let mut helper =
            PoolTestHelper::new_default_with_positions(*PRICE_RIGHT_BOUND_SQRT, &ONE_LP);
        helper.swap_success(SwapType::BuyX, dec!(1), dec!(0), dec!(1));
        helper.swap_success(
            SwapType::SellX,
            dec!(1),
            dec!("3.395647723295588275"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_one_lp_price_equal_right_bound_buy_buy_sell() {
        let mut helper =
            PoolTestHelper::new_default_with_positions(*PRICE_RIGHT_BOUND_SQRT, &ONE_LP);
        helper.swap_success(SwapType::BuyX, dec!(1), dec!(0), dec!(1));
        helper.swap_success(SwapType::BuyX, dec!(1), dec!(0), dec!(1));
        helper.swap_success(
            SwapType::SellX,
            dec!(1),
            dec!("3.395647723295588275"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_one_lp_price_greater_right_bound_buy() {
        swap_buy_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &ONE_LP,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_one_lp_price_greater_right_bound_sell() {
        swap_sell_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &ONE_LP,
            dec!(1),
            dec!("3.395647723295588275"),
            dec!(0),
        );
    }

    const TWO_LP_IDENTICAL: [LiquidityPosition; 2] = [
        LiquidityPosition {
            left_bound: TICK_LEFT_BOUND,
            right_bound: TICK_RIGHT_BOUND,
            x_amount: DEC_5,
            y_amount: DEC_5,
        },
        LiquidityPosition {
            left_bound: TICK_LEFT_BOUND,
            right_bound: TICK_RIGHT_BOUND,
            x_amount: DEC_5,
            y_amount: DEC_5,
        },
    ];

    #[test]
    fn test_swap_two_lp_identical_price_lesser_left_bound_buy() {
        swap_buy_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_identical_price_lesser_left_bound_sell() {
        swap_sell_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_identical_price_equal_left_bound_buy() {
        swap_buy_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_identical_price_equal_left_bound_sell() {
        swap_sell_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_identical_price_between_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_IDENTICAL,
            dec!(1),
            dec!("0.472987345701045226"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_identical_price_between_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_IDENTICAL,
            dec!(1),
            dec!("1.794975288895955088"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_identical_price_equal_right_bound_buy() {
        swap_buy_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_identical_price_equal_right_bound_sell() {
        swap_sell_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            dec!(1),
            dec!("3.395647723295588274"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_identical_price_greater_right_bound_buy() {
        swap_buy_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_identical_price_greater_right_bound_sell() {
        swap_sell_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            dec!(1),
            dec!("3.395647723295588274"),
            dec!(0),
        );
    }

    const TWO_LP_SEPARATE: [LiquidityPosition; 2] = [
        LiquidityPosition {
            left_bound: TICK_LEFT_BOUND,
            right_bound: TICK_LEFT_MIDDLE_BOUND,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
        LiquidityPosition {
            left_bound: TICK_RIGHT_MIDDLE_BOUND,
            right_bound: TICK_RIGHT_BOUND,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
    ];

    #[test]
    fn test_swap_two_lp_separate_price_lesser_left_bound_buy() {
        swap_buy_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(8),
            dec!("10.077821459954390409"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_lesser_left_bound_sell() {
        swap_sell_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_equal_left_bound_buy() {
        swap_buy_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(8),
            dec!("10.077821459954390409"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_equal_left_bound_sell() {
        swap_sell_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_between_left_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            dec!(8),
            dec!("5.906290476512437602"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_between_left_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            dec!(17),
            dec!("9.999999999999999997"),
            dec!("0.514023718895857471"),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_equal_left_middle_bound_buy() {
        swap_buy_success(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_equal_left_middle_bound_sell() {
        swap_sell_success(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(13),
            dec!("9.999999999999999997"),
            dec!("0.159906324597254835"),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_between_middle_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_between_middle_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            dec!(13),
            dec!("9.999999999999999997"),
            dec!("0.159906324597254835"),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_equal_right_middle_bound_buy() {
        swap_buy_success(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(35),
            dec!("9.999999999999999997"),
            dec!("0.098751676149697265"),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_equal_right_middle_bound_sell() {
        swap_sell_success(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(13),
            dec!("9.999999999999999997"),
            dec!("0.159906324597254835"),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_between_right_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            dec!(37),
            dec!("9.999999999999999997"),
            dec!("0.333859030432548145"),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_between_right_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            dec!(16),
            dec!("17.942643219402251720"),
            dec!("0.378476911229937687"),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_equal_right_bound_buy() {
        swap_buy_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_equal_right_bound_sell() {
        swap_sell_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(15),
            dec!("19.732769012728733005"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_greater_right_bound_buy() {
        swap_buy_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_separate_price_greater_right_bound_sell() {
        swap_sell_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            dec!(15),
            dec!("19.732769012728733005"),
            dec!(0),
        );
    }

    const TWO_LP_DIRECT_NEIGHBORS: [LiquidityPosition; 2] = [
        LiquidityPosition {
            left_bound: TICK_LEFT_BOUND,
            right_bound: TICK_LEFT_MIDDLE_BOUND,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
        LiquidityPosition {
            left_bound: TICK_LEFT_MIDDLE_BOUND,
            right_bound: TICK_RIGHT_BOUND,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
    ];

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_lesser_left_bound_buy() {
        swap_buy_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(8),
            dec!("10.127877267616693292"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_lesser_left_bound_sell() {
        swap_sell_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_equal_left_bound_buy() {
        swap_buy_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(8),
            dec!("10.127877267616693292"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_equal_left_bound_sell() {
        swap_sell_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_between_left_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(8),
            dec!("6.086447204658015207"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_between_left_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(17),
            dec!("9.999999999999999997"),
            dec!("0.514023718895857471"),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_equal_middle_bound_buy() {
        swap_buy_success(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(28),
            dec!("9.999999999999999997"),
            dec!("0.818540731747751365"),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_equal_middle_bound_sell() {
        swap_sell_success(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(13),
            dec!("9.999999999999999997"),
            dec!("0.159906324597254835"),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_between_right_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(30),
            dec!("9.999999999999999997"),
            dec!("0.062221262534157984"),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_between_right_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(16),
            dec!("15.547154031156184931"),
            dec!("0.105076406003378462"),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_equal_right_bound_buy() {
        swap_buy_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_equal_right_bound_sell() {
        swap_sell_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(17),
            dec!("19.999999999999999994"),
            dec!("0.480927980826017736"),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_greater_right_bound_buy() {
        swap_buy_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_direct_neighbors_price_greater_right_bound_sell() {
        swap_sell_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            dec!(17),
            dec!("19.999999999999999994"),
            dec!("0.480927980826017736"),
        );
    }

    const TWO_LP_OVERLAPPING_EXACT_LEFT: [LiquidityPosition; 2] = [
        LiquidityPosition {
            left_bound: TICK_LEFT_BOUND,
            right_bound: TICK_RIGHT_BOUND,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
        LiquidityPosition {
            left_bound: TICK_LEFT_BOUND,
            right_bound: TICK_LEFT_MIDDLE_BOUND,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
    ];

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_lesser_left_bound_buy() {
        swap_buy_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(35),
            dec!("19.999999999999999995"),
            dec!("14.371801149666482512"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_lesser_left_bound_sell() {
        swap_sell_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_equal_left_bound_buy() {
        swap_buy_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(35),
            dec!("19.999999999999999995"),
            dec!("14.371801149666482512"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_equal_left_bound_sell() {
        swap_sell_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_between_left_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(13),
            dec!("10.050952125598016069"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_between_left_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(29),
            dec!("17.458865278111122242"),
            dec!("0.217356113006735980"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_equal_middle_bound_buy() {
        swap_buy_success(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(13),
            dec!("4.523140085450459084"),
            dec!("0.705445200272935102"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_equal_middle_bound_sell() {
        swap_sell_success(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(26),
            dec!("19.999999999999999995"),
            dec!("0.319812649194509671"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_greater_middle_bound_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(9),
            dec!("2.906176684560680878"),
            dec!("0.299552544564027258"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_greater_middle_bound_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(25),
            dec!("19.999999999999999995"),
            dec!("0.501977849037746447"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_equal_right_bound_buy() {
        swap_buy_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_equal_right_bound_sell() {
        swap_sell_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(21),
            dec!("19.999999999999999995"),
            dec!("0.371801149666482512"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_greater_right_bound_buy() {
        swap_buy_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_left_bound_price_greater_right_bound_sell() {
        swap_sell_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            dec!(21),
            dec!("19.999999999999999995"),
            dec!("0.371801149666482512"),
        );
    }

    const TWO_LP_OVERLAPPING_INSIDE: [LiquidityPosition; 2] = [
        LiquidityPosition {
            left_bound: TICK_LEFT_BOUND,
            right_bound: TICK_RIGHT_BOUND,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
        LiquidityPosition {
            left_bound: TICK_LEFT_MIDDLE_BOUND,
            right_bound: TICK_RIGHT_MIDDLE_BOUND,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
    ];

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_lesser_left_bound_buy() {
        swap_buy_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(31),
            dec!("19.193649444598802867"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_lesser_left_bound_sell() {
        swap_sell_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_equal_left_bound_buy() {
        swap_buy_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(31),
            dec!("19.193649444598802867"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_equal_left_bound_sell() {
        swap_sell_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_between_left_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(34),
            dec!("17.650845586675052058"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_between_left_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(13),
            dec!("7.458865278111122244"),
            dec!("0.703332394110878508"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_equal_left_middle_bound_buy() {
        swap_buy_success(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(34),
            dec!("14.523140085450459082"),
            dec!("0.536238841348400947"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_equal_left_middle_bound_sell() {
        swap_sell_success(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(13),
            dec!("9.999999999999999997"),
            dec!("0.159906324597254835"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_between_middle_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(21),
            dec!("9.001149963350015322"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_between_middle_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(18),
            dec!("19.999999999999999995"),
            dec!("0.835049542652899557"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_equal_right_middle_bound_buy() {
        swap_buy_success(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(5),
            dec!("1.287373086613398281"),
            dec!("0.506907221866414259"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_equal_right_middle_bound_sell() {
        swap_sell_success(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(7),
            dec!("14.447580977826342641"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_between_right_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(4),
            dec!("0.932612515379344539"),
            dec!("0.580469804111805990"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_between_right_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(7),
            dec!("14.72977143891491469"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_equal_right_bound_buy() {
        swap_buy_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_equal_right_bound_sell() {
        swap_sell_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(7),
            dec!("15.903731280678732589"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_greater_right_bound_buy() {
        swap_buy_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(1),
            dec!(0),
            dec!(01),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_inside_price_greater_right_bound_sell() {
        swap_sell_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            dec!(7),
            dec!("15.903731280678732589"),
            dec!(0),
        );
    }

    // reverse position order
    const TWO_LP_OVERLAPPING_EXACT_RIGHT: [LiquidityPosition; 2] = [
        LiquidityPosition {
            left_bound: TICK_RIGHT_MIDDLE_BOUND,
            right_bound: TICK_RIGHT_BOUND,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
        LiquidityPosition {
            left_bound: TICK_LEFT_BOUND,
            right_bound: TICK_RIGHT_BOUND,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
    ];

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_lesser_left_bound_buy() {
        swap_buy_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(9),
            dec!("8.911109635781438357"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_lesser_left_bound_sell() {
        swap_sell_success(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_equal_left_bound_buy() {
        swap_buy_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(9),
            dec!("8.911109635781438357"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_equal_left_bound_sell() {
        swap_sell_success(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_between_left_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(13),
            dec!("7.715239993240959740"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_between_left_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(13),
            dec!("7.458865278111122244"),
            dec!("0.703332394110878508"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_equal_middle_bound_buy() {
        swap_buy_success(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(40),
            dec!("11.287373086613398279"),
            dec!("0.605658898016111525"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_equal_middle_bound_sell() {
        swap_sell_success(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(10),
            dec!("9.999999999999999997"),
            dec!("0.000000000000000001"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_between_right_bounds_buy() {
        swap_buy_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(41),
            dec!("10.932612515379344537"),
            dec!("0.914328834544354135"),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_between_right_bounds_sell() {
        swap_sell_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(4),
            dec!("10.896629539666365143"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_equal_right_bound_buy() {
        swap_buy_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_equal_right_bound_sell() {
        swap_sell_success(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(4),
            dec!("13.731451095218493602"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_greater_right_bound_buy() {
        swap_buy_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(1),
            dec!(0),
            dec!(1),
        );
    }

    #[test]
    fn test_swap_two_lp_overlapping_exact_right_bound_price_greater_right_bound_sell() {
        swap_sell_success(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            dec!(4),
            dec!("13.731451095218493602"),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_minimum_price_minimum_input_buy() {
        swap_buy_success(
            tick_to_price_sqrt(MIN_TICK),
            &ONE_LP,
            Decimal::ATTO,
            dec!(0),
            dec!(0),
        );
        swap_buy_success(
            tick_to_price_sqrt(MIN_TICK),
            &ONE_LP,
            dec!(0.000000000000000002),
            dec!(0.000000000000000002),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_minimum_price_minimum_input_sell() {
        swap_sell_success(
            tick_to_price_sqrt(MIN_TICK),
            &ONE_LP,
            Decimal::ATTO,
            dec!(0),
            Decimal::ATTO,
        );
        swap_sell_success(
            tick_to_price_sqrt(MIN_TICK),
            &ONE_LP,
            dec!(0.000000000000000002),
            dec!(0),
            dec!(0.000000000000000002),
        );
    }

    #[test]
    fn test_swap_minimum_price_maximum_input_buy() {
        swap_buy_success(
            tick_to_price_sqrt(MIN_TICK),
            &ONE_LP,
            MAX_SUPPLY - DEC_10,
            dec!("9.999999999999999997"),
            dec!("5708990770823839524233143854.957886870128241331"),
        );
    }

    #[test]
    fn test_swap_minimum_price_maximum_input_sell() {
        swap_sell_success(
            tick_to_price_sqrt(MIN_TICK),
            &ONE_LP,
            MAX_SUPPLY - DEC_10,
            dec!(0),
            MAX_SUPPLY - DEC_10,
        );
    }

    #[test]
    fn test_swap_maximum_price_minimum_input_buy() {
        swap_buy_success(
            tick_to_price_sqrt(MAX_TICK),
            &ONE_LP,
            Decimal::ATTO,
            dec!(0),
            Decimal::ATTO,
        );
        swap_buy_success(
            tick_to_price_sqrt(MAX_TICK),
            &ONE_LP,
            dec!(0.000000000000000002),
            dec!(0),
            dec!(0.000000000000000002),
        );
    }

    #[test]
    fn test_swap_maximum_price_minimum_input_sell() {
        swap_sell_success(
            tick_to_price_sqrt(MAX_TICK),
            &ONE_LP,
            dec!(0.000000000000000001),
            dec!(0),
            dec!(0.000000000000000001),
        );
        swap_sell_success(
            tick_to_price_sqrt(MAX_TICK),
            &ONE_LP,
            dec!(0.000000000000000002),
            dec!(0.000000000000000004),
            dec!(0),
        );
    }

    #[test]
    fn test_swap_maximum_price_maximum_input_buy() {
        swap_buy_success(
            tick_to_price_sqrt(MAX_TICK),
            &ONE_LP,
            MAX_SUPPLY - DEC_10,
            dec!(0),
            MAX_SUPPLY - DEC_10,
        );
    }

    #[test]
    fn test_swap_maximum_price_maximum_input_sell() {
        swap_sell_success(
            tick_to_price_sqrt(MAX_TICK),
            &ONE_LP,
            MAX_SUPPLY - DEC_10,
            dec!("9.999999999999999997"),
            dec!("5708990770823839524233143860.009875370600214173"),
        );
    }

    #[test]
    fn test_swap_multiple_swaps_not_crossing() {
        swap_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &ONE_LP,
            &[
                Trade {
                    type_: SwapType::BuyX,
                    input_amount: dec!(1),
                    output_expected: dec!("0.472987345701045226"),
                    remainder_expected: dec!(0),
                },
                Trade {
                    type_: SwapType::SellX,
                    input_amount: dec!(1),
                    output_expected: dec!("1.994179520493054647"),
                    remainder_expected: dec!(0),
                },
            ],
        );
    }

    #[test]
    fn test_swap_multiple_swaps_crossing_tick_buy() {
        swap_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            &[
                Trade {
                    type_: SwapType::BuyX,
                    input_amount: dec!(15),
                    output_expected: dec!("11.137187264529042571"),
                    remainder_expected: dec!(0),
                },
                Trade {
                    type_: SwapType::SellX,
                    input_amount: dec!(1),
                    output_expected: dec!("1.852220267382615589"),
                    remainder_expected: dec!(0),
                },
            ],
        );
    }

    #[test]
    fn test_swap_multiple_swaps_crossing_tick_sell() {
        swap_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            &[
                Trade {
                    type_: SwapType::SellX,
                    input_amount: dec!(2),
                    output_expected: dec!("3.354126732259704056"),
                    remainder_expected: dec!(0),
                },
                Trade {
                    type_: SwapType::BuyX,
                    input_amount: dec!(1),
                    output_expected: dec!("0.652348673683856739"),
                    remainder_expected: dec!(0),
                },
            ],
        );
    }

    #[test]
    fn test_swap_first_swap_exactly_to_tick_sell() {
        swap_success(
            pdec!("1.7").checked_sqrt().unwrap(),
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            &[
                Trade {
                    type_: SwapType::SellX,
                    input_amount: dec!("0.169878379173123052"),
                    output_expected: dec!("0.284400765840247025"),
                    remainder_expected: dec!(0),
                },
                Trade {
                    type_: SwapType::BuyX,
                    input_amount: dec!(1),
                    output_expected: dec!("0.575303373340206466"),
                    remainder_expected: dec!(0),
                },
            ],
        );
    }

    #[test]
    fn test_swap_first_swap_exactly_to_tick_buy() {
        swap_success(
            pdec!("1.6").checked_sqrt().unwrap(),
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            &[
                Trade {
                    type_: SwapType::BuyX,
                    input_amount: dec!("0.580173504224804644"),
                    output_expected: dec!("0.357215016847250127"),
                    remainder_expected: dec!(0),
                },
                Trade {
                    type_: SwapType::SellX,
                    input_amount: dec!(1),
                    output_expected: dec!("1.581820676841234776"),
                    remainder_expected: dec!(0),
                },
            ],
        );
    }

    // Edge cases ATTO swaps with high liquidity
    const POSITION_EXTREME_LEFT: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: MIN_TICK,
        right_bound: MIN_TICK + 1,
        x_amount: dec!(0.000000001),
        y_amount: dec!(0.000000001),
    }];

    const POSITION_EXTREME_RIGHT: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: MAX_TICK - 1,
        right_bound: MAX_TICK,
        x_amount: dec!(10), // TODO put as LOW as possible liquidity
        y_amount: dec!(10),
    }];

    #[test]
    fn test_swap_atto_high_liquidity_extreme_left_sell_close() {
        // price starting on nearest price != price(MIN_TICK)
        let mut helper = PoolTestHelper::new_default_with_positions(
            tick_to_price_sqrt(MIN_TICK) + PreciseDecimal::ATTO,
            &POSITION_EXTREME_LEFT,
        );

        let old_price_sqrt: Vec<PreciseDecimal> = helper
            .price_sqrt()
            .registry
            .execute_expect_success(false)
            .outputs("price_sqrt");
        helper
            .swap(helper.input_address(SwapType::SellX), Decimal::ATTO)
            .registry
            .execute_expect_success(true);
        let new_price_sqrt = helper
            .price_sqrt()
            .registry
            .execute_expect_success(false)
            .outputs("price_sqrt");

        assert_eq!(old_price_sqrt, new_price_sqrt);
    }

    #[test]
    fn test_swap_atto_high_liquidity_extreme_left_sell_cross_tick() {
        // price starting on the tick nearest to MIN_TICK
        let mut helper = PoolTestHelper::new_default_with_positions(
            tick_to_price_sqrt(MIN_TICK + 1),
            &POSITION_EXTREME_LEFT,
        );

        helper.active_tick();
        helper.price_sqrt();
        let receipt = helper.registry.execute_expect_success(false);
        let old_price_sqrt: Vec<PreciseDecimal> = receipt.outputs("price_sqrt");
        let old_active_tick: Vec<Option<i32>> = receipt.outputs("active_tick");

        helper
            .swap(helper.input_address(SwapType::SellX), Decimal::ATTO)
            .registry
            .execute_expect_success(true);

        helper.active_tick();
        helper.price_sqrt();
        let receipt = helper.registry.execute_expect_success(false);
        let new_price_sqrt: Vec<PreciseDecimal> = receipt.outputs("price_sqrt");
        let new_active_tick: Vec<Option<i32>> = receipt.outputs("active_tick");

        assert_eq!(old_price_sqrt, new_price_sqrt);
        assert_eq!(old_active_tick, new_active_tick);
    }

    const POSITIONS_PGPPGP: [LiquidityPosition; 4] = [
        // |Position| Gap |Position|Position| Gap |Position|
        LiquidityPosition {
            left_bound: -10000,
            right_bound: -5000,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
        LiquidityPosition {
            left_bound: -2500,
            right_bound: 1000,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
        LiquidityPosition {
            left_bound: 1000,
            right_bound: 2500,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
        LiquidityPosition {
            left_bound: 5000,
            right_bound: 10000,
            x_amount: DEC_10,
            y_amount: DEC_10,
        },
    ];

    fn test_swap_pgppgp_buy() {
        let trades: Vec<Trade> = vec![Trade {
            type_: SwapType::BuyX,
            input_amount: dec!("1000"),
            output_expected: dec!("39.999999999999999992") - 4 * Decimal::ATTO,
            remainder_expected: dec!("952.917123393799083657"),
        }];

        swap_success(*PRICE_LEFT_BOUND_SQRT, &POSITIONS_PGPPGP, &trades);
    }

    #[test]
    fn test_swap_pgppgp_sell() {
        let trades: Vec<Trade> = vec![Trade {
            type_: SwapType::SellX,
            input_amount: dec!("1000"),
            output_expected: dec!("39.999999999999999988"),
            remainder_expected: dec!("954.933506238128210821"),
        }];

        swap_success(*PRICE_RIGHT_BOUND_SQRT, &POSITIONS_PGPPGP, &trades);
    }

    #[test]
    fn test_swap_pgppgp_buy_sell_buy() {
        let trades: Vec<Trade> = vec![
            Trade {
                type_: SwapType::BuyX,
                input_amount: dec!("1000"),
                output_expected: dec!("39.999999999999999988"),
                remainder_expected: dec!("952.917123393799083657"),
            },
            Trade {
                type_: SwapType::SellX,
                input_amount: dec!("1000"),
                output_expected: dec!("47.082876606200916339"),
                remainder_expected: dec!("960.000000000000000008"),
            },
            Trade {
                type_: SwapType::BuyX,
                input_amount: dec!("1000"),
                output_expected: dec!("39.999999999999999988"),
                remainder_expected: dec!("952.917123393799083657"),
            },
        ];

        swap_success(*PRICE_LEFT_BOUND_SQRT, &POSITIONS_PGPPGP, &trades);
    }

    #[test]
    fn test_swap_pgppgp_sell_buy_sell() {
        let trades: Vec<Trade> = vec![
            Trade {
                type_: SwapType::SellX,
                input_amount: dec!("1000"),
                output_expected: dec!("39.999999999999999988"),
                remainder_expected: dec!("954.933506238128210821"),
            },
            Trade {
                type_: SwapType::BuyX,
                input_amount: dec!("1000"),
                output_expected: dec!("45.066493761871789175"),
                remainder_expected: dec!("960.000000000000000008"),
            },
            Trade {
                type_: SwapType::SellX,
                input_amount: dec!("1000"),
                output_expected: dec!("39.999999999999999988"),
                remainder_expected: dec!("954.933506238128210821"),
            },
        ];

        swap_success(*PRICE_RIGHT_BOUND_SQRT, &POSITIONS_PGPPGP, &trades);
    }

    // Swap minimum amount from zero liq

    #[test]
    fn test_swap_from_zero_liq_buy_sell_buy() {
        let trades: Vec<Trade> = vec![
            Trade {
                type_: SwapType::BuyX,
                input_amount: Decimal::ATTO,
                output_expected: dec!(0),
                remainder_expected: dec!(0),
            },
            Trade {
                type_: SwapType::SellX,
                input_amount: dec!("1000"),
                output_expected: dec!("29.999999999999999995") - 4 * Decimal::ATTO,
                remainder_expected: dec!("959.657348894508283606") + Decimal::ATTO,
            },
            Trade {
                type_: SwapType::BuyX,
                input_amount: dec!("1000"),
                output_expected: dec!("50.342651105491716388") - Decimal::ATTO,
                remainder_expected: dec!("948.830793641075465848") + 3 * Decimal::ATTO,
            },
        ];

        swap_success(tick_to_price_sqrt(3500), &POSITIONS_PGPPGP, &trades);
    }

    #[test]
    fn test_swap_from_zero_liq_buy_buy_sell() {
        let trades: Vec<Trade> = vec![
            Trade {
                type_: SwapType::BuyX,
                input_amount: Decimal::ATTO,
                output_expected: dec!(0),
                remainder_expected: dec!(0),
            },
            Trade {
                type_: SwapType::BuyX,
                input_amount: dec!("1000"),
                output_expected: dec!("9.999999999999999997"),
                remainder_expected: dec!("978.830793641075465846") - Decimal::ATTO,
            },
            Trade {
                type_: SwapType::SellX,
                input_amount: dec!("1000"),
                output_expected: dec!("51.169206358924534148") - 3 * Decimal::ATTO,
                remainder_expected: dec!("949.657348894508283608") + Decimal::ATTO,
            },
        ];

        swap_success(tick_to_price_sqrt(3500), &POSITIONS_PGPPGP, &trades);
    }

    #[test]
    fn test_swap_from_zero_liq_sell_buy_sell() {
        let trades: Vec<Trade> = vec![
            Trade {
                type_: SwapType::SellX,
                input_amount: Decimal::ATTO,
                output_expected: dec!(0),
                remainder_expected: Decimal::ATTO,
            },
            Trade {
                type_: SwapType::BuyX,
                input_amount: dec!("1000"),
                output_expected: dec!("9.999999999999999999") - 2 * Decimal::ATTO,
                remainder_expected: dec!("978.830793641075465843") + 2 * Decimal::ATTO,
            },
            Trade {
                type_: SwapType::SellX,
                input_amount: dec!("1000"),
                output_expected: dec!("51.169206358924534148") - 3 * Decimal::ATTO,
                remainder_expected: dec!("949.657348894508283608") + Decimal::ATTO,
            },
        ];

        swap_success(tick_to_price_sqrt(3500), &POSITIONS_PGPPGP, &trades);
    }

    #[test]
    fn test_swap_from_zero_liq_sell_sell_buy() {
        let trades: Vec<Trade> = vec![
            Trade {
                type_: SwapType::SellX,
                input_amount: Decimal::ATTO,
                output_expected: dec!(0),
                remainder_expected: Decimal::ATTO,
            },
            Trade {
                type_: SwapType::SellX,
                input_amount: dec!("1000"),
                output_expected: dec!("29.999999999999999992") - Decimal::ATTO,
                remainder_expected: dec!("959.657348894508283608") - Decimal::ATTO,
            },
            Trade {
                type_: SwapType::BuyX,
                input_amount: dec!("1000"),
                output_expected: dec!("50.342651105491716388") - Decimal::ATTO,
                remainder_expected: dec!("948.830793641075465848") + 3 * Decimal::ATTO,
            },
        ];

        swap_success(tick_to_price_sqrt(3500), &POSITIONS_PGPPGP, &trades);
    }

    #[test]
    fn test_swap_attos_buy() {
        swap_buy_success(pdec!(1), &ONE_LP, Decimal::ATTO, dec!(0), dec!(0));
        swap_buy_success(pdec!(1), &ONE_LP, Decimal::ATTO * 2, dec!(0), dec!(0));
        swap_buy_success(pdec!(1), &ONE_LP, Decimal::ATTO * 3, Decimal::ATTO, dec!(0));
    }

    #[test]
    fn test_swap_attos_sell() {
        swap_sell_success(pdec!(1), &ONE_LP, Decimal::ATTO, dec!(0), dec!(0));
        swap_sell_success(pdec!(1), &ONE_LP, Decimal::ATTO * 2, dec!(0), dec!(0));
        swap_sell_success(pdec!(1), &ONE_LP, Decimal::ATTO * 3, Decimal::ATTO, dec!(0));
    }
}
