use common::math::*;
use common::pools::SwapType;
use precision_pool::pool_math::tick_to_price_sqrt;
use precision_pool_test_helper::*;
use scrypto::prelude::*;
use scrypto_testenv::*;

const EXADECIMAL: Decimal = dec!(1_000_000_000_000_000_000);

const ONE_LP: [LiquidityPosition; 1] = [LiquidityPosition {
    left_bound: -500,
    right_bound: 500,
    x_amount: DEC_10,
    y_amount: DEC_10,
}];

const ONE_LP_LEFT: [LiquidityPosition; 1] = [LiquidityPosition {
    left_bound: -1500,
    right_bound: -500,
    x_amount: DEC_10,
    y_amount: DEC_10,
}];

const ONE_LP_RIGHT: [LiquidityPosition; 1] = [LiquidityPosition {
    left_bound: 500,
    right_bound: 1500,
    x_amount: DEC_10,
    y_amount: DEC_10,
}];

const ONE_LP_LEFT_EXTREME: [LiquidityPosition; 1] = [LiquidityPosition {
    left_bound: MIN_TICK,
    right_bound: MIN_TICK / 10,
    x_amount: DEC_10,
    y_amount: DEC_10,
}];

const ONE_LP_RIGHT_EXTREME: [LiquidityPosition; 1] = [LiquidityPosition {
    left_bound: MAX_TICK / 10,
    right_bound: MAX_TICK,
    x_amount: DEC_10,
    y_amount: DEC_10,
}];

const OVERLAPPING_LP: [LiquidityPosition; 4] = [
    LiquidityPosition {
        left_bound: -1500,
        right_bound: -500,
        x_amount: DEC_10,
        y_amount: DEC_10,
    },
    LiquidityPosition {
        left_bound: -500,
        right_bound: 500,
        x_amount: DEC_10,
        y_amount: DEC_10,
    },
    LiquidityPosition {
        left_bound: 500,
        right_bound: 1500,
        x_amount: DEC_10,
        y_amount: DEC_10,
    },
    LiquidityPosition {
        left_bound: -1000,
        right_bound: 1000,
        x_amount: DEC_10,
        y_amount: DEC_10,
    },
];

const POSITIONS_PPP: [LiquidityPosition; 3] = [
    // |Position|Position|Position|
    LiquidityPosition {
        left_bound: -1500,
        right_bound: -500,
        x_amount: DEC_10,
        y_amount: DEC_10,
    },
    LiquidityPosition {
        left_bound: -500,
        right_bound: 500,
        x_amount: DEC_10,
        y_amount: DEC_10,
    },
    LiquidityPosition {
        left_bound: 500,
        right_bound: 1500,
        x_amount: DEC_10,
        y_amount: DEC_10,
    },
];

enum Action<'a> {
    Add(&'a [LiquidityPosition]),
    SwapBuy(Decimal),
    SwapSell(Decimal),
    Remove(&'a [u64]),
}

fn delay_and_test_after_each_step(
    price_sqrt: PreciseDecimal,
    actions: &[Action],
    expected: &[&[SecondsInsideTest]],
) {
    assert_eq!(actions.len(), expected.len());

    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(price_sqrt, false);
    helper.advance_timestamp_by_seconds(600);

    let zipped: Vec<_> = actions.iter().zip(expected.iter()).collect();

    for (action, expected_batch) in zipped {
        match action {
            Action::Add(positions) => {
                helper.add_liquidity_default_batch(&positions);
            }
            Action::SwapBuy(amount) => {
                helper.swap_by_type(SwapType::BuyX, *amount);
            }
            Action::SwapSell(amount) => {
                helper.swap_by_type(SwapType::SellX, *amount);
            }
            Action::Remove(nft_ids) => {
                let nft_ids_non_fungible: IndexSet<NonFungibleLocalId> = nft_ids
                    .iter()
                    .map(|id| NonFungibleLocalId::Integer((*id).into()))
                    .collect();

                helper.remove_liquidity(nft_ids_non_fungible);
            }
        }

        helper.registry.execute_expect_success(false);
        helper.advance_timestamp_by_seconds(1);
        helper.seconds_in_position_batch_success(&expected_batch.to_vec());
    }
}

macro_rules! btree_map {
    ($($key:expr => $value:expr),* $(,)?) => {
        {
            let mut temp_map = BTreeMap::new();
            $(
                temp_map.insert($key, $value);
            )*
            temp_map
        }
    };
}

// macro_rules! btree_map {
//     ( $( ($key:expr, $value:expr) ),* $(,)? ) => {
//         {
//             let mut temp_map = BTreeMap::new();
//             $(
//                 temp_map.insert($key, $value);
//             )*
//             temp_map
//         }
//     };
// }

fn delay_and_test_after_each_step_simple(
    price_sqrt: PreciseDecimal,
    actions: &[Action],
    expected: &[BTreeMap<u64, u64>],
) {
    let mut expected_in_struct_format: Vec<Vec<SecondsInsideTest>> = Vec::new();
    for seconds_inside_test_batch in expected {
        let mut seconds_inside_test_vec: Vec<SecondsInsideTest> = Vec::new();
        for (nft_id, expected_seconds) in seconds_inside_test_batch {
            seconds_inside_test_vec.push(SecondsInsideTest {
                nft_id: (*nft_id).into(),
                seconds_in_position: *expected_seconds,
            });
        }
        expected_in_struct_format.push(seconds_inside_test_vec);
    }

    let slices: Vec<_> = expected_in_struct_format
        .iter()
        .map(|v| v.as_slice())
        .collect();
    let slice_of_slices: &[&[_]] = &slices;

    delay_and_test_after_each_step(price_sqrt, actions, slice_of_slices);
}

#[test]
fn test_seconds_in_position_swap_among_positions_basic() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&POSITIONS_PPP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(300);

    helper
        .swap_by_type(SwapType::BuyX, dec!(20))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(120);

    helper
        .swap_by_type(SwapType::SellX, dec!(20))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, dec!(20))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(180);

    helper
        .swap_by_type(SwapType::BuyX, dec!(20))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(300);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 180,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 660,
        },
        SecondsInsideTest {
            nft_id: nft_id!(3),
            seconds_in_position: 120,
        },
    ]);

    // Run a second time to confirm nothing has changed
    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 180,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 660,
        },
        SecondsInsideTest {
            nft_id: nft_id!(3),
            seconds_in_position: 120,
        },
    ]);
}

#[test]
fn test_seconds_in_position_swap_among_positions_basic_seconds_precision() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(12);

    helper
        .add_liquidity_default_batch(&POSITIONS_PPP)
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(23);

    helper
        .swap_by_type(SwapType::BuyX, dec!(20))
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(34);

    helper
        .swap_by_type(SwapType::SellX, dec!(20))
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(45);

    helper
        .swap_by_type(SwapType::SellX, dec!(20))
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(56);

    helper
        .swap_by_type(SwapType::BuyX, dec!(20))
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(67);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 56,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 135,
        },
        SecondsInsideTest {
            nft_id: nft_id!(3),
            seconds_in_position: 34,
        },
    ]);

    // Run a second time to confirm nothing has changed
    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 56,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 135,
        },
        SecondsInsideTest {
            nft_id: nft_id!(3),
            seconds_in_position: 34,
        },
    ]);
}

#[test]
fn test_seconds_in_position_remove_position_test_then_add_again_test() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&POSITIONS_PPP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(300);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 0,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 300,
        },
        SecondsInsideTest {
            nft_id: nft_id!(3),
            seconds_in_position: 0,
        },
    ]);

    helper
        .seconds_in_position(nft_id!(2))
        .registry
        .execute_expect_success(false);
    helper
        .remove_liquidity(nft_ids!(2))
        .registry
        .execute_expect_success(false);
    helper
        .seconds_in_position(nft_id!(2))
        .registry
        .execute_expect_failure(false);

    helper.advance_timestamp_by_seconds(60);
    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(120);

    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(4),
        seconds_in_position: 120,
    }]);
}

#[test]
fn test_seconds_in_position_multiple_coincident_positions_swap_test_and_remove_test() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    // Swap
    helper.swap_by_type(SwapType::BuyX, dec!(1000000));
    helper.swap_by_type(SwapType::SellX, dec!(1000000));
    helper.swap_by_type(SwapType::BuyX, dec!(1000000));
    helper.swap_by_type(SwapType::SellX, dec!(1000000));

    // Test
    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 180,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 120,
        },
        SecondsInsideTest {
            nft_id: nft_id!(3),
            seconds_in_position: 60,
        },
    ]);

    // Remove by steps, testing after each step
    helper
        .remove_liquidity(nft_ids!(1))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 180,
        },
        SecondsInsideTest {
            nft_id: nft_id!(3),
            seconds_in_position: 120,
        },
    ]);

    helper
        .remove_liquidity(nft_ids!(2))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(3),
        seconds_in_position: 180,
    }]);
}

#[test]
fn test_seconds_in_position_overlapping() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&OVERLAPPING_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, dec!(20))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, dec!(10))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, dec!(10))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, dec!(50))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, dec!(30))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, dec!(50))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 120,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 60,
        },
        SecondsInsideTest {
            nft_id: nft_id!(3),
            seconds_in_position: 180,
        },
        SecondsInsideTest {
            nft_id: nft_id!(4),
            seconds_in_position: 240,
        },
    ]);
}

#[test]
fn test_seconds_in_position_extreme_left() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(tick_to_price_sqrt(MIN_TICK / 5), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP_LEFT_EXTREME)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 60,
    }]);

    helper
        .swap_by_type(SwapType::BuyX, EXADECIMAL)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 60,
    }]);

    helper
        .swap_by_type(SwapType::SellX, Decimal::ATTO)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 60,
    }]);

    helper
        .swap_by_type(SwapType::SellX, dec!(1))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 120,
    }]);

    helper
        .swap_by_type(SwapType::SellX, EXADECIMAL)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 180,
    }]);

    helper
        .swap_by_type(SwapType::BuyX, Decimal::ATTO)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 240,
    }]);

    helper
        .swap_by_type(SwapType::BuyX, EXADECIMAL)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 240,
    }]);
}

#[test]
fn test_seconds_in_position_extreme_right() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(tick_to_price_sqrt(MAX_TICK / 5), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP_RIGHT_EXTREME)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 60,
    }]);

    helper
        .swap_by_type(SwapType::SellX, EXADECIMAL)
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 120,
    }]);

    helper
        .swap_by_type(SwapType::BuyX, Decimal::ATTO)
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 180,
    }]);

    helper
        .swap_by_type(SwapType::BuyX, EXADECIMAL)
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 180,
    }]);

    helper
        .swap_by_type(SwapType::SellX, Decimal::ATTO)
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 180,
    }]);

    helper
        .swap_by_type(SwapType::SellX, dec!(1))
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 240,
    }]);

    helper
        .swap_by_type(SwapType::SellX, EXADECIMAL)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);
    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 300,
    }]);
}

#[test]
fn test_seconds_in_position_extreme_right_old() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(tick_to_price_sqrt(MAX_TICK / 5), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP_RIGHT_EXTREME)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, EXADECIMAL)
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, Decimal::ATTO)
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, EXADECIMAL)
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, Decimal::ATTO)
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, dec!(1))
        .registry
        .execute_expect_success(true);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 240,
    }]);
}

#[test] //todo it's still equal to extended_2. Add swaps with values ATTO < value < full_step
fn test_seconds_in_position_extreme_left_2() {
    delay_and_test_after_each_step_simple(
        tick_to_price_sqrt(MIN_TICK / 5),
        &vec![
            Action::Add(&ONE_LP_LEFT_EXTREME),
            Action::SwapBuy(EXADECIMAL),
            Action::SwapSell(Decimal::ATTO),
            Action::SwapSell(dec!(1)),
            Action::SwapSell(EXADECIMAL),
            Action::SwapBuy(Decimal::ATTO),
            Action::SwapBuy(dec!(0.005)),
        ],
        &vec![
            btree_map! { 1=>1 },
            btree_map! { 1=>1 },
            btree_map! { 1=>1 },
            btree_map! { 1=>2 },
            btree_map! { 1=>3 },
            btree_map! { 1=>4 },
            btree_map! { 1=>4 },
        ],
    );
}

#[test] //todo it's still equal to extended_2. Add swaps with values ATTO < value < full_step
fn test_seconds_in_position_extreme_right_2() {
    delay_and_test_after_each_step_simple(
        tick_to_price_sqrt(MAX_TICK / 5),
        &vec![
            Action::Add(&ONE_LP_RIGHT_EXTREME),
            Action::SwapSell(EXADECIMAL),
            Action::SwapBuy(Decimal::ATTO),
            Action::SwapBuy(EXADECIMAL),
            Action::SwapSell(Decimal::ATTO),
            Action::SwapSell(dec!(1)),
            Action::SwapSell(dec!(0.005)),
        ],
        &vec![
            btree_map! { 1=>1 },
            btree_map! { 1=>2 },
            btree_map! { 1=>3 },
            btree_map! { 1=>3 },
            btree_map! { 1=>3 },
            btree_map! { 1=>4 },
            btree_map! { 1=>5 },
        ],
    );
}

#[test]
fn test_seconds_in_position_swap_till_bounds() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, dec!(1_000_000))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, dec!(1_000_000))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, dec!(1))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 180,
    }]);
}

#[test]
fn test_seconds_in_position_price_equal_left_bound() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(tick_to_price_sqrt(ONE_LP[0].left_bound), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 120,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 60,
        },
    ]);

    helper
        .remove_liquidity(nft_ids!(1))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(2),
        seconds_in_position: 120,
    }]);
}

#[test]
fn test_seconds_in_position_price_equal_right_bound() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(tick_to_price_sqrt(ONE_LP[0].right_bound), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 0,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 0,
        },
    ]);

    helper
        .remove_liquidity(nft_ids!(1))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(2),
        seconds_in_position: 0,
    }]);
}

#[test]
fn test_seconds_in_position_position_left_of_price() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);
    helper
        .add_liquidity_default_batch(&ONE_LP_LEFT)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 0,
    }]);
}

#[test]
fn test_seconds_in_position_position_right_of_price() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);
    helper
        .add_liquidity_default_batch(&ONE_LP_RIGHT)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 0,
    }]);
}

#[test]
fn test_seconds_in_position_swap_till_left_tick_then_add_position() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, dec!(1_000_000))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .add_liquidity_default_batch(&ONE_LP_LEFT)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, dec!(1))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 180,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 60,
        },
    ]);
}

#[test]
fn test_seconds_in_position_swap_till_left_tick_swap_back_then_add_position() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, dec!(1_000_000))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, Decimal::ATTO)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .add_liquidity_default_batch(&ONE_LP_LEFT)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, dec!(1))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 240,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 60,
        },
    ]);
}

#[test]
fn test_seconds_in_position_swap_till_right_tick_then_add_position() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, dec!(1_000_000))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .add_liquidity_default_batch(&ONE_LP_RIGHT)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, dec!(1))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 60,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 120,
        },
    ]);
}

#[test]
fn test_seconds_in_position_swap_till_right_tick_swap_back_then_add_position() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);

    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, dec!(1_000_000))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .add_liquidity_default_batch(&ONE_LP_RIGHT)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    // not crossing because sell did not change price due to numeric precision
    helper
        .swap_by_type(SwapType::SellX, Decimal::ATTO)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::SellX, dec!(0.1))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper
        .swap_by_type(SwapType::BuyX, dec!(1))
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.seconds_in_position_batch_success(&vec![
        SecondsInsideTest {
            nft_id: nft_id!(1),
            seconds_in_position: 120,
        },
        SecondsInsideTest {
            nft_id: nft_id!(2),
            seconds_in_position: 180,
        },
    ]);
}

#[test]
fn test_seconds_in_position_fail_if_no_position() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(tick_to_price_sqrt(ONE_LP[0].right_bound), false);

    helper
        .seconds_in_position(nft_id!(2))
        .registry
        .execute_expect_failure(false);
}

#[test]
fn test_seconds_in_position_fail_after_position_is_removed() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);

    helper
        .seconds_in_position(nft_id!(1))
        .registry
        .execute_expect_success(false);
    helper
        .remove_liquidity(nft_ids!(1))
        .registry
        .execute_expect_success(false);
    helper
        .seconds_in_position(nft_id!(1))
        .registry
        .execute_expect_failure(false);
}

#[test]
fn test_seconds_in_position_multiple_swaps_same_second() {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(pdec!(1), false);
    helper.advance_timestamp_by_seconds(600);
    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.advance_timestamp_by_seconds(60);

    helper.swap_by_type(SwapType::BuyX, Decimal::ATTO);
    helper.swap_by_type(SwapType::BuyX, Decimal::ATTO);
    helper.swap_by_type(SwapType::BuyX, Decimal::ATTO);
    helper.swap_by_type(SwapType::SellX, Decimal::ATTO);
    helper.swap_by_type(SwapType::SellX, Decimal::ATTO);
    helper.swap_by_type(SwapType::SellX, Decimal::ATTO);

    helper.registry.execute_expect_success(false);

    helper.seconds_in_position_batch_success(&vec![SecondsInsideTest {
        nft_id: nft_id!(1),
        seconds_in_position: 60,
    }]);
}
