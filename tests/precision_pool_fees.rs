#[cfg(test)]
mod precision_pool_fees {
    use common::math::*;
    use common::pools::SwapType;
    use precision_pool::pool_math::tick_to_price_sqrt;
    use precision_pool_test_helper::reverse_swap_type;
    use precision_pool_test_helper::*;
    use scrypto::prelude::*;
    use scrypto_testenv::*;

    // LIQUIDITY FEES

    static ONE_LP: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: TICK_LEFT_BOUND,
        right_bound: TICK_RIGHT_BOUND,
        x_amount: DEC_10,
        y_amount: DEC_10,
    }];

    // ADD, CLAIM (no fees claimable)
    #[test]
    fn test_claim_fees_add_claim() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_input_fee(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            FEE_RATE,
            false,
        );
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper.claim_fees_success(nft_ids!(1), dec!(0), dec!(0));
    }

    // ADD, SWAP, CLAIM (fees in swap)

    fn add_swap_claim_success(
        fee_rate_input: Decimal,
        swap_type: SwapType,
        input_amount: Decimal,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
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
        helper.swap(helper.input_address(swap_type), input_amount);
        helper.claim_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
    }

    #[test]
    fn test_claim_fees_add_swap_claim() {
        add_swap_claim_success(
            FEE_RATE,
            SwapType::BuyX,
            dec!(1),
            dec!(0),
            dec!("0.09") - Decimal::ATTO,
        );
        add_swap_claim_success(
            FEE_RATE,
            SwapType::SellX,
            dec!(1),
            dec!("0.09") - Decimal::ATTO,
            dec!(0),
        );
    }

    #[test]
    #[ignore = "Might not possible to stop on a tick without a remainder after the numeric changes."]
    fn test_claim_fee_stop_on_tick() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_input_fee(pdec!(1), FEE_RATE, false);
        helper
            .add_liquidity_default_batch(&[
                LiquidityPosition {
                    left_bound: -5,
                    right_bound: -3,
                    x_amount: DEC_5,
                    y_amount: DEC_5,
                },
                LiquidityPosition {
                    left_bound: -3,
                    right_bound: -1,
                    x_amount: DEC_5,
                    y_amount: DEC_5,
                },
                LiquidityPosition {
                    left_bound: -1,
                    right_bound: 1,
                    x_amount: DEC_5,
                    y_amount: DEC_5,
                },
                LiquidityPosition {
                    left_bound: 1,
                    right_bound: 3,
                    x_amount: DEC_5,
                    y_amount: DEC_5,
                },
                LiquidityPosition {
                    left_bound: 3,
                    right_bound: 5,
                    x_amount: DEC_5,
                    y_amount: DEC_5,
                },
            ])
            .registry
            .execute_expect_success(false);
        helper.swap_success(
            SwapType::SellX,
            dec!(16.670278159744792196),
            dec!(14.999999999999999991),
            dec!(0),
        );
    }

    // ADD, SWAP, REMOVE (fees in swap and claimed in remove)

    fn add_swap_remove_success(
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
        helper.remove_liquidity_success(nft_ids!(1), x_amount_expected, y_amount_expected);
    }

    #[test]
    fn test_claim_fees_add_swap_remove() {
        add_swap_remove_success(
            FEE_RATE,
            SwapType::BuyX,
            dec!("2.478175785363810806") + Decimal::ATTO,
            dec!("10.989999999999999997") - Decimal::ATTO * 2,
        );
        add_swap_remove_success(
            FEE_RATE,
            SwapType::SellX,
            dec!("3.896176684560680878") - Decimal::ATTO * 2,
            dec!("8.367790071541472243") + Decimal::ATTO * 2,
        );
    }

    // ADD, SWAP, REMOVE, ADD, SWAP, REMOVE

    static ONE_LP_LEFT: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: -10000,
        right_bound: -9000,
        x_amount: DEC_10,
        y_amount: DEC_10,
    }];

    static ONE_LP_RIGHT: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: 9000,
        right_bound: 10000,
        x_amount: DEC_10,
        y_amount: DEC_10,
    }];

    static ONE_LP_LEFT_EXTREME: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: MIN_TICK,
        right_bound: MIN_TICK / 10,
        x_amount: dec!(1_000_000),
        y_amount: dec!(1_000_000),
    }];

    static ONE_LP_RIGHT_EXTREME: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: MAX_TICK / 10,
        right_bound: MAX_TICK,
        x_amount: dec!(1_000_000),
        y_amount: dec!(1_000_000),
    }];

    fn add_swap_remove_add_remove_success(
        price_sqrt: PreciseDecimal,
        trades: &[(SwapType, Decimal)],
        first_position: &[LiquidityPosition],
        second_position: &[LiquidityPosition],
        x_output_expected: Decimal,
        y_output_expected: Decimal,
    ) {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_input_fee(price_sqrt, dec!("0.1"), false);
        helper
            .add_liquidity_default_batch(first_position)
            .registry
            .execute_expect_success(false);
        helper.swap(helper.input_address(trades[0].0), trades[0].1);
        helper.swap(helper.input_address(trades[1].0), trades[1].1);
        helper
            .remove_liquidity(nft_ids!(1))
            .registry
            .execute_expect_success(false);
        helper
            .add_liquidity_default_batch(second_position)
            .registry
            .execute_expect_success(false);
        helper.remove_liquidity_success(nft_ids!(2), x_output_expected, y_output_expected);
    }

    // ADD, SWAP, REMOVE, ADD, SWAP, REMOVE == SIMPLE
    #[test]
    fn test_add_swap_remove_add_swap_remove_position_center() {
        add_swap_remove_add_remove_success(
            pdec!(1),
            &vec![(SwapType::BuyX, dec!(1)), (SwapType::SellX, dec!(1))],
            &ONE_LP,
            &ONE_LP,
            dec!(9.999999999999999997),
            dec!(7.386363006711705164),
        );
    }

    #[test]
    fn test_add_swap_remove_add_swap_remove_position_left() {
        add_swap_remove_add_remove_success(
            pdec!(1),
            &vec![(SwapType::BuyX, dec!(1)), (SwapType::SellX, dec!(1))],
            &ONE_LP,
            &ONE_LP_LEFT,
            dec!(0),
            dec!(9.999999999999999997),
        );
    }

    #[test]
    fn test_add_swap_remove_add_swap_remove_position_right() {
        add_swap_remove_add_remove_success(
            pdec!(1),
            &vec![(SwapType::BuyX, dec!(1)), (SwapType::SellX, dec!(1))],
            &ONE_LP,
            &ONE_LP_RIGHT,
            dec!(9.999999999999999997),
            dec!(0),
        );
    }

    // ADD, SWAP, REMOVE, ADD, SWAP, REMOVE == price at 1 | positions in extremes
    #[test]
    fn test_add_swap_remove_add_swap_remove_position_left_extreme() {
        add_swap_remove_add_remove_success(
            pdec!(1),
            &vec![(SwapType::BuyX, dec!(1)), (SwapType::SellX, dec!(1))],
            &ONE_LP,
            &ONE_LP_LEFT_EXTREME,
            dec!(0),
            dec!(999999.999999999999999997),
        );
    }

    #[test]
    fn test_add_swap_remove_add_swap_remove_position_right_extreme() {
        add_swap_remove_add_remove_success(
            pdec!(1),
            &vec![(SwapType::BuyX, dec!(1)), (SwapType::SellX, dec!(1))],
            &ONE_LP,
            &ONE_LP_RIGHT_EXTREME,
            dec!(999999.999999999999999997),
            dec!(0),
        );
    }

    // ADD, SWAP, REMOVE, ADD, SWAP, REMOVE == positions in extremes | price in middle of extremes

    #[test]
    fn test_add_swap_remove_add_swap_remove_position_left_extreme_price_between() {
        add_swap_remove_add_remove_success(
            tick_to_price_sqrt(MIN_TICK / 5),
            &vec![
                (SwapType::BuyX, dec!(1)),
                (SwapType::SellX, dec!(1_000_000)),
            ],
            &ONE_LP_LEFT_EXTREME,
            &ONE_LP_LEFT_EXTREME,
            dec!(999999.999999999999999997),
            dec!(0.024006360636629508),
        );
    }

    #[test]
    fn test_add_swap_remove_add_swap_remove_position_right_extreme_price_between() {
        add_swap_remove_add_remove_success(
            tick_to_price_sqrt(MAX_TICK / 5),
            &vec![
                (SwapType::BuyX, dec!(1_000_000)),
                (SwapType::SellX, dec!(1)),
            ],
            &ONE_LP_RIGHT_EXTREME,
            &ONE_LP_RIGHT_EXTREME,
            dec!(89.862983447360274775),
            dec!(999999.999999999999999997),
        );
    }

    // ADD, SWAP, REMOVE, ADD, SWAP, REMOVE == positions in extremes | price in inner bound

    #[test]
    fn test_add_swap_remove_add_swap_remove_left_extreme_price_extreme_left() {
        add_swap_remove_add_remove_success(
            tick_to_price_sqrt(MIN_TICK),
            &vec![
                (SwapType::BuyX, dec!(1)),
                (SwapType::SellX, dec!(1_000_000)),
            ],
            &ONE_LP_LEFT_EXTREME,
            &ONE_LP_LEFT_EXTREME,
            dec!(999999.999999999999999997) - Decimal::ATTO * 11,
            dec!(0),
        );
    }

    #[test]
    fn test_add_swap_remove_add_swap_remove_right_extreme_price_extreme_right() {
        add_swap_remove_add_remove_success(
            tick_to_price_sqrt(MAX_TICK),
            &vec![
                (SwapType::SellX, dec!(1)),
                (SwapType::BuyX, dec!(1_000_000)),
            ],
            &ONE_LP_RIGHT_EXTREME,
            &ONE_LP_RIGHT_EXTREME,
            dec!(0),
            dec!(999999.999999999999999997) - Decimal::ATTO * 15,
        );
    }

    // ADD, SWAP, REMOVE, ADD, SWAP, REMOVE == positions in extremes | price in outer bound

    #[test]
    fn test_add_swap_remove_add_swap_remove_left_extreme_price_left_inner_bound() {
        add_swap_remove_add_remove_success(
            tick_to_price_sqrt(ONE_LP_LEFT_EXTREME[0].right_bound),
            &vec![
                (SwapType::SellX, dec!(10_000_000)),
                (SwapType::BuyX, dec!(10_000)),
            ],
            &ONE_LP_LEFT_EXTREME,
            &ONE_LP_LEFT_EXTREME,
            dec!(0),
            dec!(999999.999999999999999997),
        );
    }

    #[test]
    fn test_add_swap_remove_add_swap_remove_right_extreme_price_right_inner_bound() {
        add_swap_remove_add_remove_success(
            tick_to_price_sqrt(ONE_LP_RIGHT_EXTREME[0].left_bound),
            &vec![
                (SwapType::BuyX, dec!(10_000_000)),
                (SwapType::SellX, dec!(10_000)),
            ],
            &ONE_LP_RIGHT_EXTREME,
            &ONE_LP_RIGHT_EXTREME,
            dec!(999999.999999999999999997),
            dec!(0),
        );
    }

    // Anomaly tests. They triggered a special bug, now covered properly in concentrated_pool_swap.rs

    #[test]
    fn test_anomaly_1() {
        add_swap_remove_add_remove_success(
            tick_to_price_sqrt(MAX_TICK / 5),
            &vec![
                (SwapType::BuyX, dec!(1_000_000)),
                (SwapType::SellX, dec!(1)),
            ],
            &ONE_LP,
            &ONE_LP_RIGHT_EXTREME,
            dec!(999999.999999999999999997),
            dec!(0),
        );
    }

    #[test]
    fn test_anomaly_2() {
        add_swap_remove_add_remove_success(
            tick_to_price_sqrt(ONE_LP_RIGHT_EXTREME[0].left_bound),
            &vec![(SwapType::BuyX, dec!(1)), (SwapType::SellX, dec!(1))],
            &ONE_LP,
            &ONE_LP_RIGHT_EXTREME,
            dec!(999999.999999999999999997),
            dec!(0),
        );
    }

    #[test]
    fn test_anomaly_3() {
        add_swap_remove_add_remove_success(
            tick_to_price_sqrt(MAX_TICK),
            &vec![(SwapType::BuyX, dec!(1)), (SwapType::SellX, dec!(1))],
            &ONE_LP,
            &ONE_LP_RIGHT_EXTREME,
            dec!(999999.999999999999999997),
            dec!(0),
        );
    }

    // ADD, SWAP, CLAIM, REMOVE (fees in swap and claim, no fees anymore in remove)

    fn add_swap_claim_remove_success(
        fee_rate_input: Decimal,
        swap_type: SwapType,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
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
        helper.claim_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
        helper.remove_liquidity_success(nft_ids!(1), x_amount_expected, y_amount_expected);
    }

    #[test]
    fn test_claim_fees_add_swap_claim_remove() {
        add_swap_claim_remove_success(
            FEE_RATE,
            SwapType::BuyX,
            dec!(0),
            dec!("0.09") - Decimal::ATTO,
            dec!("2.478175785363810807"),
            dec!("10.899999999999999996"),
        );
        add_swap_claim_remove_success(
            FEE_RATE,
            SwapType::SellX,
            dec!("0.09") - Decimal::ATTO,
            dec!(0),
            dec!("3.806176684560680877"),
            dec!("8.367790071541472245"),
        );
    }

    // ADD, SWAP, CLAIM, SWAP, REMOVE (fees in claim and remove)

    fn add_swap_claim_swap_remove_success(
        fee_rate_input: Decimal,
        swap_type: SwapType,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
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
        helper.claim_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
        helper.swap(helper.input_address(reverse_swap_type(swap_type)), dec!(1));
        helper.remove_liquidity_success(nft_ids!(1), x_amount_expected, y_amount_expected);
    }

    #[test]
    fn test_claim_fees_add_swap_claim_swap_remove() {
        add_swap_claim_swap_remove_success(
            FEE_RATE,
            SwapType::BuyX,
            dec!(0),
            dec!("0.09") - Decimal::ATTO,
            dec!("3.468175785363810806") - Decimal::ATTO,
            dec!("9.104291613894967453"),
        );
        add_swap_claim_swap_remove_success(
            FEE_RATE,
            SwapType::SellX,
            dec!("0.09") - Decimal::ATTO,
            dec!(0),
            dec!("3.288259211456867775"),
            dec!("9.357790071541472243"),
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

    fn add_add_swap_claim_swap_remove_success(
        price_sqrt: PreciseDecimal,
        positions: &[LiquidityPosition],
        swap_type: SwapType,
        input_amount: Decimal,
        claim_first: bool,
        remove_first: bool,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
        x_remove_expected: Decimal,
        y_remove_expected: Decimal,
    ) {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_input_fee(price_sqrt, FEE_RATE, false);
        helper
            .add_liquidity_default_batch(positions)
            .registry
            .execute_expect_success(false);
        helper.swap(helper.input_address(swap_type), input_amount);
        helper.claim_fees_success(
            nft_ids!(if claim_first { 1 } else { 2 }),
            x_fee_expected,
            y_fee_expected,
        );
        helper.swap(
            helper.input_address(reverse_swap_type(swap_type)),
            input_amount,
        );
        helper.remove_liquidity_success(
            nft_ids!(if remove_first { 1 } else { 2 }),
            x_remove_expected,
            y_remove_expected,
        );
        // finally remove the other position to ensure there are always enough tokens in the pool
        helper
            .remove_liquidity(nft_ids!(if remove_first { 2 } else { 1 }))
            .registry
            .execute_expect_success(false);
    }

    fn add_add_swap_claim_swap_claim_success(
        price_sqrt: PreciseDecimal,
        positions: &[LiquidityPosition],
        swap_type: SwapType,
        input_amount: Decimal,
        claim_first: bool,
        claim_sequel_first: bool,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
        x_fee_sequel_expected: Decimal,
        y_fee_sequel_expected: Decimal,
    ) {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_input_fee(price_sqrt, FEE_RATE, false);
        helper
            .add_liquidity_default_batch(positions)
            .registry
            .execute_expect_success(false);
        helper.swap(helper.input_address(swap_type), input_amount);
        helper.claim_fees_success(
            nft_ids!(if claim_first { 1 } else { 2 }),
            x_fee_expected,
            y_fee_expected,
        );
        helper.swap(
            helper.input_address(reverse_swap_type(swap_type)),
            input_amount,
        );
        helper.claim_fees_success(
            nft_ids!(if claim_sequel_first { 1 } else { 2 }),
            x_fee_sequel_expected,
            y_fee_sequel_expected,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_separate_claim_first_claim_first_buy() {
        add_add_swap_claim_swap_claim_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            SwapType::BuyX,
            dec!(9),
            true,
            true,
            dec!(0),
            dec!("0.721617166174242375") - Decimal::ATTO,
            dec!("0.777716336267169833"),
            dec!(0),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_separate_claim_first_claim_second_buy() {
        add_add_swap_claim_swap_claim_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            SwapType::BuyX,
            dec!(9),
            true,
            false,
            dec!(0),
            dec!("0.721617166174242375") - Decimal::ATTO,
            dec!("0.032283663732830166") - Decimal::ATTO,
            dec!("0.088382833825757624"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_separate_claim_first_remove_first_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            SwapType::BuyX,
            dec!(9),
            true,
            true,
            dec!(0),
            dec!("0.721617166174242375") - Decimal::ATTO,
            dec!("8.554879698938868168") - Decimal::ATTO * 2,
            dec!("8.011216774288161494"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_separate_claim_first_remove_second_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            SwapType::BuyX,
            dec!(9),
            true,
            false,
            dec!(0),
            dec!("0.721617166174242375") - Decimal::ATTO,
            dec!("10.032283663732830164") - Decimal::ATTO * 2,
            dec!("0.088382833825757624"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_separate_claim_second_remove_first_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            SwapType::BuyX,
            dec!(9),
            false,
            true,
            dec!(0),
            dec!("0.088382833825757624"),
            dec!("8.554879698938868168") - Decimal::ATTO * 2,
            dec!("8.732833940462403869") - Decimal::ATTO,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_separate_claim_second_remove_second_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            SwapType::BuyX,
            dec!(9),
            false,
            false,
            dec!(0),
            dec!("0.088382833825757624"),
            dec!("10.032283663732830164") - Decimal::ATTO * 2,
            dec!(0),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_separate_claim_first_remove_first_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            SwapType::SellX,
            dec!(4),
            true,
            true,
            dec!("0.081857058663268285"),
            dec!(0),
            dec!(0),
            dec!("10.125985320142509097") - Decimal::ATTO * 2,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_separate_claim_first_remove_second_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            SwapType::SellX,
            dec!(4),
            true,
            false,
            dec!("0.081857058663268285"),
            dec!(0),
            dec!("12.211276130184434068") - Decimal::ATTO * 2,
            dec!("2.574161478432399904"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_separate_claim_second_remove_first_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            SwapType::SellX,
            dec!(4),
            false,
            true,
            dec!("0.278142941336731714") - Decimal::ATTO,
            dec!(0),
            dec!("0.081857058663268285"),
            dec!("10.125985320142509097") - Decimal::ATTO * 2,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_separate_claim_second_remove_second_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_SEPARATE,
            SwapType::SellX,
            dec!(4),
            false,
            false,
            dec!("0.278142941336731714") - Decimal::ATTO,
            dec!(0),
            dec!("11.933133188847702354") - Decimal::ATTO,
            dec!("2.574161478432399904"),
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
    fn test_claim_fees_two_lp_direct_claim_first_remove_first_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            SwapType::BuyX,
            dec!(9),
            true,
            true,
            dec!(0),
            dec!("0.721617166174242375") - Decimal::ATTO,
            dec!("8.332490429772185709") - Decimal::ATTO,
            dec!("8.184271002491985305") + Decimal::ATTO,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_direct_claim_first_remove_second_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            SwapType::BuyX,
            dec!(9),
            true,
            false,
            dec!(0),
            dec!("0.721617166174242375") - Decimal::ATTO,
            dec!("10.052500870020710387") - Decimal::ATTO * 2,
            dec!("0.088382833825757624"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_direct_claim_second_remove_first_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            SwapType::BuyX,
            dec!(9),
            false,
            true,
            dec!(0),
            dec!("0.088382833825757624"),
            dec!("8.332490429772185709") - Decimal::ATTO,
            dec!("8.905888168666227680"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_direct_claim_second_remove_second_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            SwapType::BuyX,
            dec!(9),
            false,
            false,
            dec!(0),
            dec!("0.088382833825757624"),
            dec!("10.052500870020710387") - Decimal::ATTO * 2,
            dec!(0),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_direct_claim_first_remove_first_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            SwapType::SellX,
            dec!(6),
            true,
            true,
            dec!("0.090353538077660114"),
            dec!(0),
            dec!(0),
            dec!("10.138109269040807645") - Decimal::ATTO * 2,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_direct_claim_first_remove_second_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            SwapType::SellX,
            dec!(6),
            true,
            false,
            dec!("0.090353538077660114"),
            dec!(0),
            dec!("5.151036433367449699") - Decimal::ATTO,
            dec!("4.420798040551115875") - Decimal::ATTO,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_direct_claim_second_remove_first_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            SwapType::SellX,
            dec!(6),
            false,
            true,
            dec!("0.449646461922339885") - Decimal::ATTO,
            dec!(0),
            dec!("0.090353538077660114"),
            dec!("10.138109269040807645") - Decimal::ATTO * 2,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_direct_claim_second_remove_second_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_DIRECT_NEIGHBORS,
            SwapType::SellX,
            dec!(6),
            false,
            false,
            dec!("0.449646461922339885") - Decimal::ATTO,
            dec!(0),
            dec!("4.701389971445109814"),
            dec!("4.420798040551115875") - Decimal::ATTO,
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
    fn test_claim_fees_two_lp_overlapping_exact_left_claim_first_remove_first_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            SwapType::BuyX,
            dec!(17),
            true,
            true,
            dec!(0),
            dec!("0.808382833825757624"),
            dec!("12.452897321616144076") - Decimal::ATTO,
            dec!("5.886203283362663667"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_left_claim_first_remove_second_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            SwapType::BuyX,
            dec!(17),
            true,
            false,
            dec!(0),
            dec!("0.808382833825757624"),
            dec!("8.710588811031187917"),
            dec!("8.613170457311418689"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_left_claim_second_remove_first_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            SwapType::BuyX,
            dec!(17),
            false,
            true,
            dec!(0),
            dec!("0.721617166174242375"),
            dec!("12.452897321616144076") - Decimal::ATTO,
            dec!("6.694586117188421291"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_left_claim_second_remove_second_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            SwapType::BuyX,
            dec!(17),
            false,
            false,
            dec!(0),
            dec!("0.721617166174242375"),
            dec!("8.710588811031187917"),
            dec!("7.891553291137176314"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_left_claim_first_remove_first_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            SwapType::SellX,
            dec!(20),
            true,
            true,
            dec!("0.869376418445925435"),
            dec!(0),
            dec!("2.525152667739110944"),
            dec!("11.748818085164336570"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_left_claim_first_remove_second_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            SwapType::SellX,
            dec!(20),
            true,
            false,
            dec!("0.869376418445925435"),
            dec!(0),
            dec!("0.930623581554074564"),
            dec!("10.847904196284034487") - Decimal::ATTO * 2,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_left_claim_second_remove_first_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            SwapType::SellX,
            dec!(20),
            false,
            true,
            dec!("0.930623581554074564"),
            dec!(0),
            dec!("3.394529086185036379"),
            dec!("11.748818085164336570"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_left_claim_second_remove_second_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_LEFT,
            SwapType::SellX,
            dec!(20),
            false,
            false,
            dec!("0.930623581554074564"),
            dec!(0),
            dec!(0),
            dec!("10.847904196284034487") - Decimal::ATTO * 2,
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
    fn test_claim_fees_two_lp_overlapping_inside_claim_first_remove_first_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            SwapType::BuyX,
            dec!(38),
            true,
            true,
            dec!(0),
            dec!("1.303079364107546584") + Decimal::ATTO,
            dec!("24.298486351055908539") - Decimal::ATTO,
            dec!(0),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_inside_claim_first_remove_second_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            SwapType::BuyX,
            dec!(38),
            true,
            false,
            dec!(0),
            dec!("1.303079364107546584") + Decimal::ATTO,
            dec!("10.999999999999999997") - Decimal::ATTO,
            dec!("2.116920635892453415") - Decimal::ATTO,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_inside_claim_second_remove_first_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            SwapType::BuyX,
            dec!(38),
            false,
            true,
            dec!(0),
            dec!("2.116920635892453415") - Decimal::ATTO,
            dec!("24.298486351055908539") - Decimal::ATTO,
            dec!("1.303079364107546584") + Decimal::ATTO,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_inside_claim_second_remove_second_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_LEFT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            SwapType::BuyX,
            dec!(38),
            false,
            false,
            dec!(0),
            dec!("2.116920635892453415") - Decimal::ATTO,
            dec!("10.999999999999999997") - Decimal::ATTO,
            dec!(0),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_inside_claim_first_remove_first_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            SwapType::SellX,
            dec!(23),
            true,
            true,
            dec!("0.951865874099431592") - Decimal::ATTO * 2,
            dec!(0),
            dec!("0.709430982930136798") + Decimal::ATTO,
            dec!("11.770000000000000001") - Decimal::ATTO * 2,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_inside_claim_first_remove_second_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            SwapType::SellX,
            dec!(23),
            true,
            false,
            dec!("0.951865874099431592") - Decimal::ATTO * 2,
            dec!(0),
            dec!("0.472384265638007278"),
            dec!("10.999999999999999997") - Decimal::ATTO,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_inside_claim_second_remove_first_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            SwapType::SellX,
            dec!(23),
            false,
            true,
            dec!("0.472384265638007278"),
            dec!(0),
            dec!("1.661296857029568390") - Decimal::ATTO,
            dec!("11.770000000000000001") - Decimal::ATTO * 2,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_inside_claim_second_remove_second_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            SwapType::SellX,
            dec!(23),
            false,
            false,
            dec!("0.472384265638007278"),
            dec!(0),
            dec!(0),
            dec!("10.999999999999999997") - Decimal::ATTO,
        );
    }

    // Full range buy/sell

    #[test]
    fn test_claim_fees_two_lp_overlapping_inside_claim_second_remove_second_sell_full_range() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_INSIDE,
            SwapType::SellX,
            dec!(100),
            false,
            false,
            dec!("0.472384265638007278"),
            dec!(0),
            dec!(0),
            dec!("10.999999999999999997") - Decimal::ATTO,
        );
    }

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
    fn test_claim_fees_two_lp_overlapping_exact_right_claim_first_remove_first_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            SwapType::BuyX,
            dec!(15),
            true,
            true,
            dec!(0),
            dec!("0.441252566999666202"),
            dec!("12.569630642151904534") - Decimal::ATTO,
            dec!("1.310646917050536117") + Decimal::ATTO,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_right_claim_first_remove_second_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            SwapType::BuyX,
            dec!(15),
            true,
            false,
            dec!(0),
            dec!("0.441252566999666202"),
            dec!("10.311305355295585185") - Decimal::ATTO,
            dec!("0.908747433000333797"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_right_claim_second_remove_first_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            SwapType::BuyX,
            dec!(15),
            false,
            true,
            dec!(0),
            dec!("0.908747433000333797"),
            dec!("12.569630642151904534") - Decimal::ATTO,
            dec!("1.751899484050202320"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_right_claim_second_remove_second_buy() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            SwapType::BuyX,
            dec!(15),
            false,
            false,
            dec!(0),
            dec!("0.908747433000333797"),
            dec!("10.311305355295585185") - Decimal::ATTO,
            dec!(0),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_right_claim_first_remove_first_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            SwapType::SellX,
            dec!(6),
            true,
            true,
            dec!("0.261857058663268285") + Decimal::ATTO,
            dec!(0),
            dec!("1.162674803600778977"),
            dec!("9.793465645029926623") - Decimal::ATTO,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_right_claim_first_remove_second_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            SwapType::SellX,
            dec!(6),
            true,
            false,
            dec!("0.261857058663268285") + Decimal::ATTO,
            dec!(0),
            dec!("12.745001196266492306") - Decimal::ATTO * 2,
            dec!("0.945703586673142560"),
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_right_claim_second_remove_first_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            SwapType::SellX,
            dec!(6),
            false,
            true,
            dec!("0.278142941336731714") - Decimal::ATTO,
            dec!(0),
            dec!("1.424531862264047262") + Decimal::ATTO,
            dec!("9.793465645029926623") - Decimal::ATTO,
        );
    }

    #[test]
    fn test_claim_fees_two_lp_overlapping_exact_right_claim_second_remove_second_sell() {
        add_add_swap_claim_swap_remove_success(
            *PRICE_BETWEEN_RIGHT_BOUNDS_SQRT,
            &TWO_LP_OVERLAPPING_EXACT_RIGHT,
            SwapType::SellX,
            dec!(6),
            false,
            false,
            dec!("0.278142941336731714") - Decimal::ATTO,
            dec!(0),
            dec!("12.466858254929760591"),
            dec!("0.945703586673142560"),
        );
    }

    // TOTAL_FEES

    fn add_swap_claim_total_swap_claim_total_success(
        fee_rate_input: Decimal,
        swap_type: SwapType,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
        x_total_fee_expected: Decimal,
        y_total_fee_expected: Decimal,
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
        helper.claim_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
        helper.total_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
        helper.swap(helper.input_address(swap_type), dec!(1));
        helper.claim_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
        helper.total_fees_success(nft_ids!(1), x_total_fee_expected, y_total_fee_expected);
        helper.claim_fees_success(nft_ids!(1), dec!(0), dec!(0));
    }

    #[test]
    fn test_total_fees() {
        add_swap_claim_total_swap_claim_total_success(
            dec!(0.1),
            SwapType::SellX,
            dec!(0.089999999999999999),
            dec!(0),
            dec!(0.179999999999999999),
            dec!(0),
        );
        add_swap_claim_total_swap_claim_total_success(
            dec!(0.1),
            SwapType::BuyX,
            dec!(0),
            dec!(0.089999999999999999),
            dec!(0),
            dec!(0.179999999999999999),
        );
    }

    // TOKEN DIVISIBILITY

    fn helper_with_divisibility(
        price_sqrt: PreciseDecimal,
        fee_rate_input: Decimal,
    ) -> PoolTestHelper {
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
        helper.instantiate_default_with_input_fee(price_sqrt, fee_rate_input, false);
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper
    }

    fn divisibility_success(
        price_sqrt: PreciseDecimal,
        fee_rate_input: Decimal,
        swap_type: SwapType,
        swap_input: Decimal,
        swap_output_expected: Decimal,
        swap_remainder_expected: Decimal,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
        swap_second_input: Decimal,
        swap_second_output_expected: Decimal,
        swap_second_remainder_expected: Decimal,
        x_total_fee_expected: Decimal,
        y_total_fee_expected: Decimal,
        x_remove_expected: Decimal,
        y_remove_expected: Decimal,
    ) {
        let mut helper = helper_with_divisibility(price_sqrt, fee_rate_input);
        helper.swap_success(
            swap_type,
            swap_input,
            swap_output_expected,
            swap_remainder_expected,
        );
        helper.claim_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
        helper.total_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
        helper.swap_success(
            swap_type,
            swap_second_input,
            swap_second_output_expected,
            swap_second_remainder_expected,
        );
        helper.claim_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
        helper.total_fees_success(nft_ids!(1), x_total_fee_expected, y_total_fee_expected);
        helper.claim_fees_success(nft_ids!(1), dec!(0), dec!(0));
        helper.remove_liquidity_success(nft_ids!(1), x_remove_expected, y_remove_expected)
    }

    #[test]
    fn test_lower_divisibility() {
        divisibility_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            dec!(0.1),
            SwapType::SellX,
            dec!(1.000001),
            dec!(1.632208283946474),
            dec!(0),
            dec!(0.09),
            dec!(0),
            dec!(1.000001),
            dec!(1.353857359942493),
            dec!(0),
            dec!(0.180001),
            dec!(0),
            dec!(4.706174),
            dec!(7.01393435611103),
        );
        divisibility_success(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            dec!(0.1),
            SwapType::BuyX,
            dec!(1.000000000000001),
            dec!(0.428),
            dec!(0),
            dec!(0),
            dec!(0.09),
            dec!(1.000000000000001),
            dec!(0.388104),
            dec!(0),
            dec!(0),
            dec!(0.180000000000001),
            dec!(2.090071),
            dec!(11.799999999999995),
        );
    }

    fn helper_with_liquidity(
        price_sqrt: PreciseDecimal,
        fee_rate_input: Decimal,
    ) -> PoolTestHelper {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_input_fee(price_sqrt, fee_rate_input, false);
        helper
            .add_liquidity_default_batch(&ONE_LP)
            .registry
            .execute_expect_success(false);
        helper
    }

    fn add_swap_success(
        fee_rate_input: Decimal,
        swap_type: SwapType,
        input_amount: Decimal,
        output_expected: Decimal,
        remainder_expected: Decimal,
    ) -> PoolTestHelper {
        let mut helper = helper_with_liquidity(pdec!(1), fee_rate_input);
        helper.swap_success(swap_type, input_amount, output_expected, remainder_expected);
        helper
    }

    fn add_swap_failure(fee_rate_input: Decimal, swap_type: SwapType, input_amount: Decimal) {
        let mut helper = helper_with_liquidity(pdec!(1), fee_rate_input);
        helper.swap_failure(swap_type, input_amount);
    }

    #[test]
    fn test_swap_attos_buy() {
        add_swap_failure(dec!(0.1), SwapType::BuyX, Decimal::ATTO);
        add_swap_success(
            dec!(0.1),
            SwapType::BuyX,
            Decimal::ATTO * 2,
            dec!(0),
            dec!(0),
        );
        add_swap_success(
            dec!(0.1),
            SwapType::BuyX,
            Decimal::ATTO * 3,
            dec!(0),
            dec!(0),
        );
        add_swap_success(
            dec!(0.1),
            SwapType::BuyX,
            Decimal::ATTO * 4,
            Decimal::ATTO,
            dec!(0),
        );
    }

    #[test]
    fn test_swap_attos_sell() {
        add_swap_failure(dec!(0.1), SwapType::SellX, Decimal::ATTO);
        add_swap_success(
            dec!(0.1),
            SwapType::SellX,
            Decimal::ATTO * 2,
            dec!(0),
            dec!(0),
        );
        add_swap_success(
            dec!(0.1),
            SwapType::SellX,
            Decimal::ATTO * 3,
            dec!(0),
            dec!(0),
        );
        add_swap_success(
            dec!(0.1),
            SwapType::SellX,
            Decimal::ATTO * 4,
            Decimal::ATTO,
            dec!(0),
        );
    }

    #[test]
    fn test_swap_input_amount_net_rounding() {
        let mut helper = add_swap_success(
            dec!(0.1),
            SwapType::BuyX,
            dec!(1.000000000000000001),
            dec!(0.859200506444526745), // ensures ceiling of input fee total
            dec!(0),
        );
        // ensures ceiling of protocol fee
        helper.remove_liquidity_success(
            nft_ids!(1),
            dec!(9.140799493555473252),
            dec!(8.447210849065006032),
        );
    }

    #[test]
    fn test_swap_input_amount_net_rounding_divisibility() {
        let mut helper = helper_with_divisibility(pdec!(1), dec!(0.1));
        helper.swap_success(
            SwapType::SellX,
            dec!(1.000001),
            dec!(0.859199587264989), // ensures ceiling of input fee total
            dec!(0),
        );
        // ensures ceiling of protocol fee
        helper.remove_liquidity_success(nft_ids!(1), dec!(10.989996), dec!(6.598009770357847));
    }

    const ONE_LP_2: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: -500,
        right_bound: 500,
        x_amount: DEC_10,
        y_amount: DEC_10,
    }];

    const ONE_LP_LEFT_2: [LiquidityPosition; 1] = [LiquidityPosition {
        left_bound: -1500,
        right_bound: -500,
        x_amount: DEC_10,
        y_amount: DEC_10,
    }];

    #[test]
    fn test_swap_till_left_tick_then_add_position_test() {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default_with_input_fee(pdec!(1), dec!(0.1), false);

        helper
            .add_liquidity_default_batch(&ONE_LP_2)
            .registry
            .execute_expect_success(false);
        helper
            .swap_by_type(SwapType::SellX, dec!(1))
            .registry
            .execute_expect_success(false);
        helper
            .swap_by_type(SwapType::BuyX, dec!(2))
            .registry
            .execute_expect_success(true);

        helper.claim_fees_success(
            nft_ids!(1),
            dec!(0.09) - Decimal::ATTO,
            dec!(0.18) - Decimal::ATTO,
        );
        helper
            .remove_liquidity(nft_ids!(1))
            .registry
            .execute_expect_success(false);

        helper
            .add_liquidity_default_batch(&ONE_LP_2)
            .registry
            .execute_expect_success(true);

        helper
            .swap_by_type(SwapType::SellX, dec!(1_000_000))
            .registry
            .execute_expect_success(true);

        helper
            .add_liquidity_default_batch(&ONE_LP_LEFT_2)
            .registry
            .execute_expect_success(true);

        helper
            .swap_by_type(SwapType::SellX, dec!(2))
            .registry
            .execute_expect_success(true);
        helper
            .swap_by_type(SwapType::BuyX, dec!(1))
            .registry
            .execute_expect_success(true);

        // #2# is ONE_LP (#1# position is already removed)
        helper.claim_fees_success(
            nft_ids!(2),
            dec!(1.02303561585665759) - Decimal::ATTO,
            dec!(0),
        );
        helper
            .remove_liquidity(nft_ids!(2))
            .registry
            .execute_expect_success(false);

        // #3# is ONE_LP_LEFT
        helper.claim_fees_success(
            nft_ids!(3),
            dec!(0.18) - Decimal::ATTO,
            dec!(0.09) - Decimal::ATTO,
        );
        helper
            .remove_liquidity(nft_ids!(3))
            .registry
            .execute_expect_success(false);
    }

    #[test]
    fn test_sell_cost() {
        // price starting on the tick nearest to MIN_TICK

        // let n_sets_of_8_positions = 8 * 2;
        let n_sets_of_8_positions = 10;

        let swap_type = SwapType::SellX;

        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(
            match swap_type {
                SwapType::BuyX => tick_to_price_sqrt(-n_sets_of_8_positions * 8),
                SwapType::SellX => pdec!(1),
            },
            false,
        );

        for i in 0..n_sets_of_8_positions {
            let mut positions: Vec<LiquidityPosition> = Vec::new();
            for j in 0..8 {
                positions.push(LiquidityPosition {
                    left_bound: 0 - i * 8 - j - 1,
                    right_bound: 0 - i * 8 - j,
                    x_amount: dec!(0.0001),
                    y_amount: dec!(0.0001),
                });
            }
            helper
                .add_liquidity_default_batch(&positions)
                .registry
                .execute_expect_success(false);
        }
        helper
            .swap(helper.input_address(swap_type), dec!(100000))
            .registry
            .execute_expect_success(true);

        for i in 0..n_sets_of_8_positions {
            for j in 0..8 {
                let index = (i * 8 + j + 1) as u64;
                println!("Remove NFT={}", index);
                helper
                    .remove_liquidity(nft_ids!(index))
                    .registry
                    .execute_expect_success(false);
            }
        }
    }

    // CLAIMABLE_FEES

    fn add_swap_claimable_claim_swap_claimable_claim_success(
        fee_rate_input: Decimal,
        swap_type: SwapType,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
        x_second_fee_expected: Decimal,
        y_second_fee_expected: Decimal,
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
        helper.claimable_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
        helper.claim_fees_success(nft_ids!(1), x_fee_expected, y_fee_expected);
        helper.swap(helper.input_address(swap_type), dec!(1));
        helper.claimable_fees_success(nft_ids!(1), x_second_fee_expected, y_second_fee_expected);
        helper.claim_fees_success(nft_ids!(1), x_second_fee_expected, y_second_fee_expected);
    }

    #[test]
    fn test_claimable_fees() {
        add_swap_claimable_claim_swap_claimable_claim_success(
            dec!(0.1),
            SwapType::SellX,
            dec!(0.089999999999999999),
            dec!(0),
            dec!(0.089999999999999999),
            dec!(0),
        );
        add_swap_claimable_claim_swap_claimable_claim_success(
            dec!(0.1),
            SwapType::BuyX,
            dec!(0),
            dec!(0.089999999999999999),
            dec!(0),
            dec!(0.089999999999999999),
        );
    }

    fn add_swap_claimable_claim_multiple_success(
        fee_rate_input: Decimal,
        swap_type: SwapType,
        x_first_fee_expected: Decimal,
        y_first_fee_expected: Decimal,
        x_second_fee_expected: Decimal,
        y_second_fee_expected: Decimal,
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

        let (x_fee_expected, y_fee_expected) = (
            x_first_fee_expected + x_second_fee_expected,
            y_first_fee_expected + y_second_fee_expected,
        );
        helper.claimable_fees_success(nft_ids!(1), x_first_fee_expected, y_first_fee_expected);
        helper.claimable_fees_success(nft_ids!(2), x_second_fee_expected, y_second_fee_expected);
        helper.claimable_fees_success(nft_ids!(1, 2), x_fee_expected, y_fee_expected);
        helper.claim_fees_success(nft_ids!(1, 2), x_fee_expected, y_fee_expected);
    }

    #[test]
    fn test_claimable_fees_multiple() {
        add_swap_claimable_claim_multiple_success(
            dec!(0.1),
            SwapType::SellX,
            dec!(0.349297801821240794),
            dec!(0),
            dec!(0.550702198178759205),
            dec!(0),
        );
        add_swap_claimable_claim_multiple_success(
            dec!(0.1),
            SwapType::BuyX,
            dec!(0),
            dec!(0.124946456941939466),
            dec!(0),
            dec!(0.775053543058060533),
        );
    }
}
