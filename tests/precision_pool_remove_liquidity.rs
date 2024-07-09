#[cfg(test)]
mod precision_pool_remove_liquidity {
    use common::pools::SwapType;
    use precision_pool_test_helper::*;
    use scrypto::prelude::*;
    use scrypto_testenv::*;

    static ONE_LP: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: TICK_LEFT_BOUND,
        right_bound: TICK_RIGHT_BOUND,
        x_amount: DEC_10,
        y_amount: DEC_10,
    }];

    // ADD, REMOVE
    #[test]
    fn test_remove_liquidity_add_remove() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(*PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT, true);
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper.remove_liquidity_success(
            nft_ids!(1),
            dec!("2.906176684560680878"),
            dec!("9.999999999999999997"),
        );
    }

    // ADD, REMOVE, SWAP
    #[test]
    fn test_remove_liquidity_add_remove_swap() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(*PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT, true);
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper.remove_liquidity(nft_ids!(1));
        helper.swap_success(SwapType::SellX, dec!(1), dec!(0), dec!(1));
    }

    // ADD, SWAP, REMOVE
    #[test]
    fn test_remove_liquidity_add_swap_remove() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(*PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT, true);
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper.swap_x_default(dec!(1));
        helper.remove_liquidity_success(
            nft_ids!(1),
            dec!("3.906176684560680877"),
            dec!("8.205024711104044909"),
        );
    }

    // ADD, SWAP, SWAP, REMOVE
    #[test]
    fn test_remove_liquidity_add_swap_swap_remove() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(*PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT, true);
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper.swap_x_default(dec!(1));
        helper.swap_y_default(dec!("1.794975288895955090"));
        helper.remove_liquidity_success(
            nft_ids!(1),
            dec!("2.906176684560680878"),
            dec!("9.999999999999999998"),
        );
    }

    // ADD, REMOVE, ADD, SWAP
    #[test]
    fn test_remove_liquidity_add_remove_add_swap() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(*PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT, true);
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper.remove_liquidity_success(
            nft_ids!(1),
            dec!("2.906176684560680878"),
            dec!("9.999999999999999997"),
        );
        helper.add_liquidity_default_batch(&ONE_LP);
        helper.swap_success(
            SwapType::SellX,
            dec!(1),
            dec!("1.794975288895955088"),
            dec!(0),
        );
    }

    // ADD, ADD, REMOVE, REMOVE
    #[test]
    fn test_remove_liquidity_add_add_remove_remove() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(*PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT, true);
        helper
            .add_liquidity_default_batch(&TWO_LP_OVERLAPPING_INSIDE)
            .registry
            .execute_expect_success(false);
        helper.remove_liquidity_success(
            nft_ids!(1),
            dec!("2.906176684560680878"),
            dec!("9.999999999999999997"),
        );
        helper.remove_liquidity_success(
            nft_ids!(2),
            dec!("7.723327129193566959"),
            dec!("9.999999999999999997"),
        );
    }

    // ADD, REMOVE - MAX SUPPLY
    #[test]
    #[ignore]
    fn test_remove_liquidity_add_remove_max_supply() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(*PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT, true);
        helper
            .add_liquidity_default(TICK_LEFT_BOUND, TICK_RIGHT_BOUND, MAX_SUPPLY, MAX_SUPPLY)
            .registry
            .execute_expect_success(false);
        helper.remove_liquidity_success(
            nft_ids!(1),
            dec!("1659133587054035186118531952.395783209047671783"), // 1659133587054035186118531952.395783210177039528
            dec!("5708990770823839524233143877.797980545530986493"), // 5708990770823839524233143877.797980538462497204
        );
    }

    /*

    ADD (outer) with price in the middle
    ADD (inner)
    REMOVE (inner)
    SELL beyond left middle bound (active tick is now in inconsistent state if not updated)
    ADD with left bound between price and left middle bound
    BUY beyond at least left bound of newly added liquidity (or beyond previous active tick)

    */

    #[test]
    fn test_remove_liquidity_update_active_tick() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(*PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT, true);
        helper
            .add_liquidity_default_batch(&TWO_LP_OVERLAPPING_INSIDE)
            .registry
            .execute_expect_success(false);
        helper.remove_liquidity_success(
            nft_ids!(2),
            dec!("7.723327129193566959"),
            dec!("9.999999999999999997"),
        );
        helper.swap_success(
            SwapType::SellX,
            dec!(2),
            dec!("3.256154509029914474"),
            dec!(0),
        );
        helper.add_liquidity_default_batch(&TWO_LP_OVERLAPPING_INSIDE[1..]);
        helper.swap_success(
            SwapType::BuyX,
            dec!(10),
            dec!("5.751791266274584555"),
            dec!(0),
        );
    }

    // All following tests will follow this pattern
    // ADD, ADD, REMOVE, SWAP, REMOVE
    // checking for the output the swap and final remove liquidity
    // If possible swap tries to move price into the interval with previously overlapping liquidity

    const TWO_LP_IDENTICAL: [LiquidityPosition; 2] = [
        LiquidityPosition {
            left_bound: TICK_LEFT_BOUND,
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
    fn test_remove_liquidity_two_lp_identical_price_lesser_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            true,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
            dec!("7.723369154450054775"),
            dec!("0.999999999999999998"),
        );

        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            false,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
            dec!("7.723369154450054775"),
            dec!("0.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_identical_price_lesser_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );

        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_identical_price_equal_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            true,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
            dec!("7.723369154450054775"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            false,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
            dec!("7.723369154450054775"),
            dec!("0.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_identical_price_equal_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );

        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_identical_price_between_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_IDENTICAL,
            true,
            dec!(1),
            dec!("0.472987345701045226"),
            dec!(0),
            dec!("2.433189338859635652"),
            dec!("10.999999999999999996"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_IDENTICAL,
            false,
            dec!(1),
            dec!("0.472987345701045226"),
            dec!(0),
            dec!("2.433189338859635652"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_identical_price_between_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_IDENTICAL,
            false,
            dec!(1),
            dec!("1.794975288895955088"),
            dec!(0),
            dec!("3.906176684560680877"),
            dec!("8.205024711104044909"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_IDENTICAL,
            true,
            dec!(1),
            dec!("1.794975288895955088"),
            dec!(0),
            dec!("3.906176684560680877"),
            dec!("8.205024711104044909"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_identical_price_equal_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_identical_price_equal_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            true,
            dec!(1),
            dec!("3.395647723295588275"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("6.604352276704411722"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            false,
            dec!(1),
            dec!("3.395647723295588275"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("6.604352276704411722"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_identical_price_greater_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_identical_price_greater_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            true,
            dec!(1),
            dec!("3.395647723295588275"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("6.604352276704411722"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_IDENTICAL,
            false,
            dec!(1),
            dec!("3.395647723295588275"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("6.604352276704411722"),
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
    fn test_remove_liquidity_two_lp_separate_price_lesser_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
            dec!("9.635071773865548053"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!("2.377220101169762399"),
            dec!(0),
            dec!("7.622779898830237598"),
            dec!("0.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_lesser_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_equal_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
            dec!("9.635071773865548053"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!("2.377220101169762399"),
            dec!(0),
            dec!("7.622779898830237598"),
            dec!("0.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_equal_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_between_left_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
            dec!("9.635071773865548053"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!("0.96204502900250587"),
            dec!(0),
            dec!("4.657704376752038062"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_between_left_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!("0.962233804730264013"),
            dec!(0),
            dec!("6.619749405754543932"),
            dec!("9.037766195269735984"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_equal_left_middle_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
            dec!("9.635071773865548053"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_equal_left_middle_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_between_middle_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
            dec!("9.635071773865548053"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_between_middle_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_equal_right_middle_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
            dec!("9.635071773865548053"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_equal_right_middle_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_between_right_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!("0.331325429979230457"),
            dec!(0),
            dec!("9.668674570020769540"),
            dec!("8.942643219402251722"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_between_right_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!("2.946431966049474108"),
            dec!(0),
            dec!("10.999999999999999996"),
            dec!("4.996211253352777614"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_equal_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_equal_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!("4.077208587634587224"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("5.922791412365412773"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_greater_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_separate_price_greater_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            true,
            dec!(1),
            dec!("4.077208587634587224"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("5.922791412365412773"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_SEPARATE,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
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
    fn test_remove_liquidity_two_lp_direct_neighbors_price_lesser_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!("0.592408121542722592"),
            dec!(0),
            dec!("9.407591878457277405"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!("2.377220101169762399"),
            dec!(0),
            dec!("7.622779898830237598"),
            dec!("0.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_lesser_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_equal_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!("0.592408121542722592"),
            dec!(0),
            dec!("9.407591878457277405"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!("2.377220101169762399"),
            dec!(0),
            dec!("7.622779898830237598"),
            dec!("0.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_equal_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_between_left_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!("0.592408121542722592"),
            dec!(0),
            dec!("9.407591878457277405"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!("0.96204502900250587"),
            dec!(0),
            dec!("4.657704376752038062"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_between_left_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!("0.962233804730264013"),
            dec!(0),
            dec!("6.619749405754543932"),
            dec!("9.037766195269735984"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_equal_middle_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!("0.592408121542722592"),
            dec!(0),
            dec!("9.407591878457277405"),
            dec!(0.999999999999999998),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_equal_middle_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_between_right_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!("0.491836793780859286"),
            dec!(0),
            dec!("9.508163206219140711"),
            dec!("6.547154031156184933"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_between_right_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!("1.935743429518425968"),
            dec!(0),
            dec!("10.999999999999999996"),
            dec!("3.611410601637758965"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_equal_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_equal_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!("3.809635381675562966"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("6.190364618324437031"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_greater_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_direct_neighbors_price_greater_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            true,
            dec!(1),
            dec!("3.809635381675562966"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("6.190364618324437031"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
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
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_lesser_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!("2.377220101169762399"),
            dec!(0),
            dec!("7.622779898830237598"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
            dec!("7.723369154450054775"),
            dec!("0.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_lesser_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_equal_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!("2.377220101169762399"),
            dec!(0),
            dec!("7.622779898830237598"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!("2.276630845549945222"),
            dec!(0),
            dec!("7.723369154450054775"),
            dec!("0.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_equal_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_between_left_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!("0.96204502900250587"),
            dec!(0),
            dec!("4.657704376752038062"),
            dec!("10.999999999999999996"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!("0.949794605971203801"),
            dec!(0),
            dec!("9.050205394028796196"),
            dec!("8.458865278111122243"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_between_left_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!("0.962233804730264013"),
            dec!(0),
            dec!("6.619749405754543932"),
            dec!("9.037766195269735984"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!("0.949979814002112995"),
            dec!(0),
            dec!("10.999999999999999996"),
            dec!("6.508885464109009248"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_equal_middle_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!("0.576147379865013072"),
            dec!(0),
            dec!("3.946992705585446012"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_equal_middle_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("5.523140085450459083"),
            dec!("8.483256813615744545"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_between_right_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!("0.472987345701045226"),
            dec!(0),
            dec!("2.433189338859635652"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_between_right_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("8.483256813615744545"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!("1.794975288895955088"),
            dec!(0),
            dec!("3.906176684560680877"),
            dec!("8.205024711104044909"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_equal_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_equal_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(2),
            dec!("2.808716693164989217"),
            dec!(0),
            dec!("1.999999999999999998"),
            dec!("7.19128330683501078"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(3),
            dec!("6.862003765886179368"),
            dec!(0),
            dec!("2.999999999999999998"),
            dec!("3.137996234113820629"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_greater_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_left_bound_price_greater_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            true,
            dec!(2),
            dec!("2.808716693164989217"),
            dec!(0),
            dec!("1.999999999999999998"),
            dec!("7.19128330683501078"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            false,
            dec!(3),
            dec!("6.862003765886179368"),
            dec!(0),
            dec!("2.999999999999999998"),
            dec!("3.137996234113820629"),
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
    fn test_remove_liquidity_two_lp_overlapping_inside_price_lesser_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!("0.598516037834045211"),
            dec!(0),
            dec!("9.401483962165954786"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(6),
            dec!("7.537837942237830886"),
            dec!(0),
            dec!("2.462162057762169111"),
            dec!("5.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_lesser_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_equal_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!("0.598516037834045211"),
            dec!(0),
            dec!("9.401483962165954786"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(6),
            dec!("7.537837942237830886"),
            dec!(0),
            dec!("2.462162057762169111"),
            dec!("5.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_equal_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_between_left_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!("0.598516037834045211"),
            dec!(0),
            dec!("9.401483962165954786"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(6),
            dec!("4.557001471262850059"),
            dec!(0),
            dec!("5.442998528737149938"),
            dec!("13.458865278111122243"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_between_left_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!("0.949979814002112995"),
            dec!(0),
            dec!("10.999999999999999996"),
            dec!("6.508885464109009248"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_equal_left_middle_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!("0.598516037834045211"),
            dec!(0),
            dec!("9.401483962165954786"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!("0.576147379865013072"),
            dec!(0),
            dec!("3.946992705585446012"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_equal_left_middle_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!("1.516743186384255452"),
            dec!(0),
            dec!("5.523140085450459083"),
            dec!("8.483256813615744545"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_between_middle_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!("0.495438582724065106"),
            dec!(0),
            dec!("7.227888546469501853"),
            dec!("10.999999999999999996"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!("0.472987345701045226"),
            dec!(0),
            dec!("2.433189338859635652"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_between_middle_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!("1.963838556657293178"),
            dec!(0),
            dec!("8.723327129193566958"),
            dec!("8.036161443342706819"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!("0.5"),
            dec!("0.945974691402090451"),
            dec!(0),
            dec!("3.406176684560680877"),
            dec!("9.054025308597909546"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_equal_right_middle_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!("0.346025488324086850"),
            dec!(0),
            dec!("0.941347598289311431"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_equal_right_middle_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!("2.563992229417155407"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("7.43600777058284459"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!("2.319604094196194228"),
            dec!(0),
            dec!("2.28737308661339828"),
            dec!("7.680395905803805769"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_between_right_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!("0.312994611506147406"),
            dec!(0),
            dec!("0.619617903873197133"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_between_right_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!("2.563992229417155407"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("7.43600777058284459"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!("2.510579620408396213"),
            dec!(0),
            dec!("1.932612515379344538"),
            dec!("7.489420379591603784"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_equal_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_equal_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!("2.563992229417155407"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("7.43600777058284459"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!("3.395647723295588275"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("6.604352276704411722"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_greater_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_inside_price_greater_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            true,
            dec!(1),
            dec!("2.563992229417155407"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("7.43600777058284459"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            false,
            dec!(1),
            dec!("3.395647723295588275"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("6.604352276704411722"),
        );
    }

    // reverse position order
    const TWO_LP_OVERLAPPING_EXACT_RIGHT: [LiquidityPosition; 2] = [
        LiquidityPosition {
            left_bound: TICK_LEFT_BOUND,
            right_bound: TICK_RIGHT_BOUND,
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
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_lesser_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
            dec!("9.635071773865548053"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(9),
            dec!("8.910649338085273303"),
            dec!(0),
            dec!("1.089350661914726694"),
            dec!("8.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_lesser_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LESSER_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_equal_left_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
            dec!("9.635071773865548053"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(9),
            dec!("8.910649338085273303"),
            dec!(0),
            dec!("1.089350661914726694"),
            dec!("8.999999999999999998"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_equal_left_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_LEFT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_between_left_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
            dec!("9.635071773865548053"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(13),
            dec!("7.710695874849511526"),
            dec!(0),
            dec!("2.289304125150488471"),
            dec!("20.458865278111122243"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_between_left_bounds_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(1),
            dec!("0.949979814002112995"),
            dec!(0),
            dec!("10.999999999999999996"),
            dec!("6.508885464109009248"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_equal_middle_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!("0.364928226134451944"),
            dec!(0),
            dec!("9.635071773865548053"),
            dec!("0.999999999999999998"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(1),
            dec!("0.346025488324086850"),
            dec!(0),
            dec!("0.941347598289311431"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_equal_middle_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!("9.999999999999999997"),
            dec!(0),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_MIDDLE_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(1),
            dec!("2.319604094196194228"),
            dec!(0),
            dec!("2.28737308661339828"),
            dec!("7.680395905803805769"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_between_right_bounds_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!("0.331325429979230457"),
            dec!(0),
            dec!("9.668674570020769540"),
            dec!("8.942643219402251722"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(1),
            dec!("0.312994611506147406"),
            dec!(0),
            dec!("0.619617903873197133"),
            dec!("10.999999999999999996"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_between_right_bounds_sell()
    {
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!("2.946431966049474108"),
            dec!(0),
            dec!("10.999999999999999996"),
            dec!("4.996211253352777614"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!("0.2"),
            dec!("0.577484660555675865"),
            dec!(0),
            dec!("1.132612515379344538"),
            dec!("9.422515339444324132"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_equal_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_equal_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!("4.077208587634587224"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("5.922791412365412773"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!("0.5"),
            dec!("1.931838857841354515"),
            dec!(0),
            dec!("0.499999999999999998"),
            dec!("8.068161142158645482"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_greater_right_bound_buy() {
        remove_liquidity_default_scenario_buy(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
        remove_liquidity_default_scenario_buy(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!(1),
            dec!(0),
            dec!(1),
            dec!(0),
            dec!("9.999999999999999997"),
        );
    }

    #[test]
    fn test_remove_liquidity_two_lp_overlapping_exact_right_bound_price_greater_right_bound_sell() {
        remove_liquidity_default_scenario_sell(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            true,
            dec!(1),
            dec!("4.077208587634587224"),
            dec!(0),
            dec!("0.999999999999999998"),
            dec!("5.922791412365412773"),
        );
        remove_liquidity_default_scenario_sell(
            *PRICE_GREATER_RIGHT_BOUND_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            false,
            dec!("0.5"),
            dec!("1.931838857841354515"),
            dec!(0),
            dec!("0.499999999999999998"),
            dec!("8.068161142158645482"),
        );
    }

    // REMOVABLE_LIQUIDITY

    fn add_swap_removable_remove_success(
        fee_rate_input: Decimal,
        swap_type: SwapType,
        x_amount_expected: Decimal,
        y_amount_expected: Decimal,
    ) {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_input_fee(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            fee_rate_input,
            false,
        );
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper.swap(helper.input_address(swap_type), dec!(1));
        helper.removable_liquidity_success(
            nft_ids!(1),
            x_amount_expected,
            y_amount_expected,
            dec!(1),
        );
        helper.remove_liquidity_success(nft_ids!(1), x_amount_expected, y_amount_expected);
    }

    #[test]
    fn test_removable_liquidity() {
        add_swap_removable_remove_success(
            dec!(0.1),
            SwapType::SellX,
            dec!(3.896176684560680876),
            dec!(8.367790071541472245),
        );
        add_swap_removable_remove_success(
            dec!(0.1),
            SwapType::BuyX,
            dec!(2.478175785363810807),
            dec!(10.989999999999999995),
        );
    }

    fn add_swap_removable_remove_multiple_success(
        fee_rate_input: Decimal,
        swap_type: SwapType,
        x_first_amount_expected: Decimal,
        y_first_amount_expected: Decimal,
        x_second_amount_expected: Decimal,
        y_second_amount_expected: Decimal,
    ) {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_input_fee(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            fee_rate_input,
            false,
        );
        helper
            .add_liquidity_default_batch(&TWO_LP_OVERLAPPING_INSIDE)
            .registry
            .execute_expect_success(false);
        helper.swap(helper.input_address(swap_type), dec!(10));

        let (x_amount_expected, y_amount_expected) = (
            x_first_amount_expected + x_second_amount_expected,
            y_first_amount_expected + y_second_amount_expected,
        );
        helper.removable_liquidity_success(
            nft_ids!(1),
            x_first_amount_expected,
            y_first_amount_expected,
            dec!(1),
        );
        helper.removable_liquidity_success(
            nft_ids!(2),
            x_second_amount_expected,
            y_second_amount_expected,
            dec!(1),
        );
        helper.removable_liquidity_success(
            nft_ids!(1, 2),
            x_amount_expected,
            y_amount_expected,
            dec!(1),
        );
        helper.remove_liquidity_success(nft_ids!(1, 2), x_amount_expected, y_amount_expected);
    }

    #[test]
    fn test_removable_remove_multiple() {
        add_swap_removable_remove_multiple_success(
            dec!(0.1),
            SwapType::SellX,
            dec!(6.748452504594329615),
            dec!(5.006368481716003411),
            dec!(13.781051309159918219),
            dec!(0),
        );
        add_swap_removable_remove_multiple_success(
            dec!(0.1),
            SwapType::BuyX,
            dec!(2.323054722457962828),
            dec!(11.374411026361334132),
            dec!(4.106171798629102106),
            dec!(18.525588973638665861),
        );
    }

    #[test]
    fn test_removable_liquidity_minimum_removable_fraction() {
        removable_liquidity_with_remove_hook(
            nft_ids!(1),
            dec!(9.999999999999999997),
            dec!(7.457210849065006033),
            dec!(0.9),
        )
    }
}
