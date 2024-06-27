use std::cmp::{max, min};
use std::ops::{Add, Sub};

use precision_pool_hooks::AfterSwapState;
use common::math::*;
use common::pools::SwapType;
use scrypto::prelude::*;

use crate::constants::{DIVISIBILITY_UNITS, TICK_BASE_SQRT};
use crate::pool::{Tick, TickOutside};
use scrypto_avltree::IterMutControl;

use crate::constants::*;

/// Calculates the total number of ticks based on the specified tick spacing.
///
/// This function determines the number of discrete price levels (ticks) that can exist within the range
/// defined by `MAX_TICK`, given a specific `spacing`. The formula used ensures that the number of ticks
/// covers the entire price range symmetrically around zero.
///
/// # Arguments
/// * `spacing` - The distance between each tick.
///
/// # Returns
/// The total number of ticks as a `u32`.
fn number_of_ticks(spacing: u32) -> u32 {
    2 * ((MAX_TICK as u32) / spacing) + 1
}

/// Computes the maximum liquidity allowed per tick for a given tick spacing.
///
/// This function divides the total maximum liquidity (`MAX_LIQUIDITY`) by the number of ticks
/// to find the maximum amount of liquidity that can be allocated to each tick. This ensures
/// that liquidity is distributed evenly across all ticks and helps in managing the liquidity concentration.
///
/// # Arguments
/// * `spacing` - The distance between each tick.
///
/// # Returns
/// The maximum liquidity per tick as a `PreciseDecimal`.
pub fn max_liquidity_per_tick(spacing: u32) -> PreciseDecimal {
    MAX_LIQUIDITY / number_of_ticks(spacing)
}

/// Converts a tick index to its corresponding price square root.
///
/// This function calculates the square root of the price for a given tick index. Converting the discrete tick index into a continuous price
/// representation used in various pool calculations.
///
/// # Arguments
/// * `tick` - The tick index.
///
/// # Returns
/// The price square root as a `PreciseDecimal`.
pub fn tick_to_price_sqrt(tick: i32) -> PreciseDecimal {
    TICK_BASE_SQRT.checked_powi(tick.into()).unwrap()
}

/// Aligns a given tick index to the nearest valid tick that is a multiple of the specified tick spacing.
///
/// # Arguments
/// * `tick` - The tick index to align.
/// * `spacing` - The tick spacing that defines the grid.
///
/// # Returns
/// The aligned tick index as an `i32`.
pub fn align_tick(tick: i32, spacing: u32) -> i32 {
    let spacing = spacing as i32;

    (tick / spacing) * spacing
}

/// Calculates the allowed amounts of tokens `x` and `y` that can be added to the pool based on the current
/// pool price and price bounds.
///
/// It accounts for precision errors by adjusting the amounts with a margin derived from the divisibility of each token.
///
/// # Arguments
/// - `x_amount`: The initial amount of token `x`.
/// - `x_divisibility`: The divisibility of token `x`, used for precision adjustments.
/// - `y_amount`: The initial amount of token `y`.
/// - `y_divisibility`: The divisibility of token `y`, used for precision adjustments.
/// - `price_sqrt`: The current square root of the pool's price.
/// - `price_left_bound_sqrt`: The lower bound square root of the price for the liquidity position.
/// - `price_right_bound_sqrt`: The upper bound square root of the price for the liquidity position.
///
/// # Returns
/// A tuple containing:
/// - `liquidity`: The maximum liquidity that can be added given the token amounts and price bounds.
/// - `x_amount_allowed`: The adjusted amount of token `x` that can be added.
/// - `y_amount_allowed`: The adjusted amount of token `y` that can be added.
pub fn allowed_amounts(
    x_amount: Decimal,
    x_divisibility: u8,
    y_amount: Decimal,
    y_divisibility: u8,
    price_sqrt: PreciseDecimal,
    price_left_bound_sqrt: PreciseDecimal,
    price_right_bound_sqrt: PreciseDecimal,
) -> (PreciseDecimal, Decimal, Decimal) {
    /*
    Deduct two units of divisibility from the initial token amounts:
     - One unit to account for rounding up in the final calculations.
     - Another unit to correct for potential precision errors.

    We add PreciseDecimal::ATTO to intermediate calculations when necessary to ensure:
    - The computed liquidity is consistently less than or equal to the theoretical value with perfect precision.
    - The calculated allowed token amounts are always greater than or equal to their theoretical values with perfect precision.
    */

    let x_precision_margin = divisibility_unit(x_divisibility) * dec!(2);
    let y_precision_margin = divisibility_unit(y_divisibility) * dec!(2);

    let x_amount_safe =
        PreciseDecimal::from(subtract_precision_margin(x_amount, x_precision_margin));
    let y_amount_safe =
        PreciseDecimal::from(subtract_precision_margin(y_amount, y_precision_margin));

    // Determine the scale and liquidity based on the price bounds.
    if price_sqrt <= price_left_bound_sqrt {
        let x_scale = x_scale_safe(price_left_bound_sqrt, price_right_bound_sqrt);
        let x_liquidity = x_amount_safe / x_scale;
        return (x_liquidity, x_amount, dec!(0));
    }

    if price_sqrt >= price_right_bound_sqrt {
        let y_scale = y_scale_safe(price_left_bound_sqrt, price_right_bound_sqrt);
        let y_liquidity = y_amount_safe / y_scale;
        return (y_liquidity, dec!(0), y_amount);
    }

    let x_scale = x_scale_safe(price_sqrt, price_right_bound_sqrt);
    let x_liquidity = x_amount_safe / x_scale;

    let y_scale = y_scale_safe(price_left_bound_sqrt, price_sqrt);
    let y_liquidity = y_amount_safe / y_scale;

    let liquidity = min(x_liquidity, y_liquidity);

    /*
    Removing 1 divisibility unit from the input for precision correction is enough because:
    - PreciseDecimal::ATTO * x_scale + 2 * PreciseDecimal::ATTO < Decimal::ATTO
    - PreciseDecimal::ATTO * y_scale + 2 * PreciseDecimal::ATTO < Decimal::ATTO
    */
    let x_amount_allowed = (liquidity * x_scale + Decimal::ATTO).ceil_to(x_divisibility);
    let y_amount_allowed = (liquidity * y_scale + Decimal::ATTO).ceil_to(y_divisibility);

    // Adjust the allowed amounts to ensure they are within the precision margin.
    (
        liquidity,
        adjust_within_margin(x_amount, x_amount_allowed, x_precision_margin),
        adjust_within_margin(y_amount, y_amount_allowed, y_precision_margin),
    )
}

/// Calculates the safe scale for token `x` based on the price bounds.
///
/// This function determines the scale factor for token `x` to ensure that the liquidity calculations
/// remain within the specified price bounds.
///
/// # Arguments
/// - `lower_price_sqrt`: The lower bound square root of the price.
/// - `upper_price_sqrt`: The upper bound square root of the price.
///
/// # Returns
/// The scale factor as a `PreciseDecimal`.
fn x_scale_safe(
    lower_price_sqrt: PreciseDecimal,
    upper_price_sqrt: PreciseDecimal,
) -> PreciseDecimal {
    pdec!(1) / lower_price_sqrt + PreciseDecimal::ATTO - pdec!(1) / upper_price_sqrt
}

/// Calculates the safe scale for token `y` based on the price bounds.
///
/// This function determines the scale factor for token `y` to ensure that the liquidity
/// calculations remain within the specified price bounds.
///
/// # Arguments
/// - `lower_price_sqrt`: The lower bound square root of the price.
/// - `upper_price_sqrt`: The upper bound square root of the price.
///
/// # Returns
/// The scale factor as a `PreciseDecimal`.
fn y_scale_safe(
    lower_price_sqrt: PreciseDecimal,
    upper_price_sqrt: PreciseDecimal,
) -> PreciseDecimal {
    // Both price_sqrt are treated as constants in the current adding of liquidity and therefore
    // no arithmetic correction is necessary.
    upper_price_sqrt - lower_price_sqrt
}

/// Returns the divisibility unit for a given divisibility.
///
/// # Arguments
/// - `divisibility`: The divisibility level.
///
/// # Returns
/// The corresponding precision unit as a `Decimal`.
fn divisibility_unit(divisibility: u8) -> Decimal {
    DIVISIBILITY_UNITS[divisibility as usize]
}

/// Subtracts a precision margin from an amount, ensuring the result is non-negative.
///
/// # Arguments
/// - `amount`: The original amount.
/// - `margin`: The margin to subtract.
///
/// # Returns
/// The adjusted amount as a `Decimal`.
fn subtract_precision_margin(amount: Decimal, margin: Decimal) -> Decimal {
    max(amount - margin, dec!(0))
}

/// Adjusts an amount to ensure it is within a specified margin of an allowed amount.
///
/// This function ensures that the actual amount does not exceed the allowed amount by more than the
/// specified margin, adjusting if necessary.
///
/// # Arguments
/// - `amount`: The original amount.
/// - `allowed_amount`: The maximum allowed amount.
/// - `margin`: The margin within which the amount must fall.
///
/// # Returns
/// The adjusted amount as a `Decimal`.
fn adjust_within_margin(amount: Decimal, allowed_amount: Decimal, margin: Decimal) -> Decimal {
    assert!(
        amount >= allowed_amount,
        "Amount needs to be greater or equal than allowed_amount"
    );
    if amount - allowed_amount <= margin {
        return amount;
    }
    allowed_amount
}

/// Calculates the withdrawable amounts of x and y tokens based on the provided liquidity and price bounds.
/// This function is crucial for determining how much of each token can be safely removed from a liquidity position
/// without violating the constraints set by the current and boundary prices.
///
/// # Arguments
///
/// - `liquidity`: The total liquidity available in the position.
/// - `price_sqrt`: The square root of the current pool price.
/// - `price_left_bound_sqrt`: The square root of the lower price boundary of the liquidity position.
/// - `price_right_bound_sqrt`: The square root of the upper price boundary of the liquidity position.
/// - `x_divisibility`: The divisibility of token x, used for rounding down the withdrawable amount.
/// - `y_divisibility`: The divisibility of token y, used for rounding down the withdrawable amount.
///
/// # Returns
///
/// A tuple containing the withdrawable amounts of token `x` and token `y`.
///
pub fn remove_amounts(
    liquidity: PreciseDecimal,
    price_sqrt: PreciseDecimal,
    price_left_bound_sqrt: PreciseDecimal,
    price_right_bound_sqrt: PreciseDecimal,
    x_divisibility: u8,
    y_divisibility: u8,
) -> (Decimal, Decimal) {
    // When the current price is below the lower bound, all liquidity can be withdrawn as token x.
    if price_sqrt <= price_left_bound_sqrt {
        let x_amount = max(
            liquidity / price_left_bound_sqrt
                - (liquidity / price_right_bound_sqrt + PreciseDecimal::ATTO),
            pdec!(0),
        );
        return (x_amount.floor_to(x_divisibility), dec!(0));
    }

    // When the current price is above the upper bound, all liquidity can be withdran as token y.
    if price_sqrt >= price_right_bound_sqrt {
        let y_amount = liquidity * (price_right_bound_sqrt - price_left_bound_sqrt);
        return (dec!(0), y_amount.floor_to(y_divisibility));
    }

    // When the current price is within the bounds, calculate the withdrawable amounts for both tokens.
    let x_amount = max(
        liquidity / price_sqrt - (liquidity / price_right_bound_sqrt + PreciseDecimal::ATTO),
        pdec!(0),
    );
    let y_amount = liquidity * (price_sqrt - price_left_bound_sqrt);

    (
        x_amount.floor_to(x_divisibility),
        y_amount.floor_to(y_divisibility),
    )
}

/// Calculates the net input amount after deducting liquidity provider and protocol fees.
///
/// Determining the actual amount of tokens that can be used in further transactions after accounting for
/// the fees associated with the transaction. It ensures transparency in fee distribution and protects against
/// negative input amounts due to rounding errors in fee calculations.
///
/// # Arguments
/// * `input_amount` - The total amount of tokens before fees are applied.
/// * `input_fee_rate` - The rate at which the input fee is applied to the `input_amount`.
/// * `fee_protocol_share` - The fraction of the total fee that goes to the protocol.
/// * `divisibility` - The precision to which the amounts should be rounded, typically the token divisibility.
///
/// # Returns
/// * `(Decimal, Decimal, Decimal)` - A tuple containing:
///   - `input_amount_net`: The amount of tokens after fees are deducted.
///   - `input_fee_lp`: The fee amount allocated to the liquidity provider.
///   - `protocol_fee_input`: The fee amount allocated to the protocol.
pub fn input_amount_net(
    input_amount: Decimal,
    input_fee_rate: Decimal,
    fee_protocol_share: Decimal,
    divisibility: u8,
) -> (Decimal, Decimal, Decimal) {
    let input_amount_gross: PreciseDecimal = input_amount.into();
    /*
    Valid pre-conditions:
      `0 <= input_fee_rate <= 1`
      => `0 <= input_amount_gross * input_fee_rate <= input_amount_gross`
      => ceiling to the 18th decimal can lead to `input_fee_lp > input_amount_gross` (with input_fee_rate = 1)
         but only if input_amount_gross has non-zero digits afte the 18th decimal place
         otherwise it is guaranteed that `input_fee_lp <= input_amount_gross`
      => since input_amount_gross is converted from Decimal (with only 18 decimal places) it is strictly true that:
         `input_fee_lp < input_amount_gross`
    Therefore:
      input_amount_net >= 0
    In other words the calculated input_amount_net is always positve or equal zero.
    */
    let input_fee_total: Decimal = (input_amount_gross * input_fee_rate).ceil_to(divisibility);
    // Protocol fees are being rounded down in favour of liquidity provider fees
    let protocol_fee_input = (input_fee_total * fee_protocol_share).floor_to(divisibility);
    let input_fee_lp = input_fee_total - protocol_fee_input;
    let input_amount_net: Decimal = input_amount - input_fee_total;

    assert!(
        input_amount_net > Decimal::ZERO,
        "Input amount net needs to be positive!"
    );

    (input_amount_net, input_fee_lp, protocol_fee_input)
}

/// Calculates the new price after a swap operation, adjusting for precision errors.
///
/// This function determines the new square root price (`price_sqrt`) after a swap has occurred.
/// It accounts for precision errors by adjusting the input amount slightly before performing the calculation.
/// The adjustment ensures that the resulting price is slightly less favorable, compensating forthe inherent
/// imprecision in fixed-point arithmetic.
///
/// # Arguments
/// * `swap_type` - The type of swap (`BuyX` or `SellX`) which determines the direction of the price change.
/// * `liquidity` - The current liquidity in the pool, affecting how much the price changes per unit of input.
/// * `price_sqrt` - The square root of the current price.
/// * `input_amount` - The amount of the input token.
/// * `input_divisibility` - The divisibility of the input token, used to adjust the input amount for precision.
///
/// # Returns
/// * `PreciseDecimal` - The new square root price after the swap.
pub fn new_price(
    swap_type: SwapType,
    liquidity: PreciseDecimal,
    price_sqrt: PreciseDecimal,
    input_amount: Decimal,
    input_divisibility: u8,
) -> PreciseDecimal {
    /*
    For BuyX transactions, the adjusted new price should be marginally less favorable, specifically, it should be
    slightly lower than the precise arithmetic result yet still above the current price. Conversely, for SellX
    transactions, the adjusted new price should be marginally higher than the precise arithmetic result but remain
    below the current price.
    To account for the precision error in input_step, we subtract one divisibility unit from input_amount.
    */
    let input_amount = max(
        PreciseDecimal::from(input_amount) - divisibility_unit(input_divisibility),
        pdec!(0),
    );
    match swap_type {
        SwapType::BuyX => max(input_amount / liquidity + price_sqrt, price_sqrt),
        SwapType::SellX => min(
            (liquidity * price_sqrt + PreciseDecimal::ATTO)
                / (liquidity + input_amount * price_sqrt)
                + PreciseDecimal::ATTO,
            price_sqrt,
        ),
    }
}

/// Adjusts the current liquidity based on the swap type and the change in liquidity.
///
/// # Arguments
/// * `swap_type` - The type of swap (`BuyX` or `SellX`) which determines the direction of liquidity adjustment.
/// * `liquidity` - The current liquidity of the pool.
/// * `delta_liquidity` - The amount by which the liquidity needs to be adjusted.
///
/// # Returns
/// * `PreciseDecimal` - The new liquidity after applying the adjustment.
fn new_liquidity(
    swap_type: SwapType,
    liquidity: PreciseDecimal,
    delta_liquidity: PreciseDecimal,
) -> PreciseDecimal {
    match swap_type {
        SwapType::BuyX => liquidity + delta_liquidity,
        SwapType::SellX => liquidity - delta_liquidity,
    }
}

/// Calculates the input amount required to move the price from `price_sqrt` to `price_next_sqrt`.
///
/// This function ensures that the input amount respects the divisibility of the token and adheres to precision
/// constraints inherent in fixed-point arithmetic operations. The result is always rounded up to the nearest
/// valid divisibility unit, ensuring that the price movement is achievable with the given input amount by always
/// rounding in favour of the pool.
///
/// # Arguments
/// * `swap_type` - The type of swap (`BuyX` or `SellX`) which determines the direction of the price movement.
/// * `liquidity` - The current liquidity in the (sub) pool, which affects how much the price moves per unit of input.
/// * `price_sqrt` - The square root of the current price.
/// * `price_next_sqrt` - The square root of the target price after the swap.
/// * `input_divisibility` - The divisibility of the input token, used to round the result.
///
/// # Returns
/// * `Decimal` - The input amount, adjusted for divisibility and rounded up.
///
/// # Panics
/// Panics if the absolute difference calculation fails, which should not occur with valid input data.
pub fn input_step(
    swap_type: SwapType,
    liquidity: PreciseDecimal,
    price_sqrt: PreciseDecimal,
    price_next_sqrt: PreciseDecimal,
    input_divisibility: u8,
) -> Decimal {
    /*
    // Liquidity, price_sqrt, and price_next_sqrt are treated as constants within a single step.
    // The precision error is limited to 10^36 due to the use of PreciseDecimal units.

    // To address precision errors when amounts or prices are nearly identical, the input amount in
    // new_price is reduced by one divisibility unit and rounded down to the nearest divisibility unit.
    // This adjustment guarantees that the input_step is always at least as large as needed but does not
    // exceed the specified input amount.

    // The result is guaranteed to be greater than or equal to the precise numerical result. The precision
    // error is limited to one divisibility unit.
    */
    (match swap_type {
        SwapType::BuyX => {
            // abs is calculated on exact result, precision correction is independent of abs
            liquidity * (price_sqrt - price_next_sqrt).checked_abs().unwrap() + PreciseDecimal::ATTO
        }
        SwapType::SellX => {
            (liquidity / price_sqrt - liquidity / price_next_sqrt)
                .checked_abs()
                .unwrap()
                + PreciseDecimal::ATTO
        }
    })
    .ceil_to(input_divisibility)
}

/// Calculates the output amount for a given swap step, ensuring the result is always less than or equal to the
/// exact numeric result.
///
/// This function is crucial for maintaining the integrity of the pool's state by preventing overestimation of
/// output amounts during swaps. It uses the liquidity and the square roots of the prices at the current and
/// next ticks to compute the output amount, adjusted for divisibility.
///
/// # Arguments
/// * `swap_type` - The type of swap (`BuyX` or `SellX`) which determines the calculation method.
/// * `liquidity` - The liquidity available at the current tick.
/// * `price_sqrt` - The square root of the price at the current tick.
/// * `price_next_sqrt` - The square root of the price at the next tick.
/// * `output_divisibility` - The divisibility of the output token, used to round the result.
///
/// # Returns
/// * `Decimal` - The calculated output amount, rounded down to ensure it does not exceed the exact result.
///
/// # Panics
/// Panics if the absolute difference calculation fails, which should not occur with valid input data.
pub fn output_step(
    swap_type: SwapType,
    liquidity: PreciseDecimal,
    price_sqrt: PreciseDecimal,
    price_next_sqrt: PreciseDecimal,
    output_divisibility: u8,
) -> Decimal {
    (match swap_type {
        SwapType::BuyX => {
            // For BuyX, calculate the difference in liquidity per price unit, adjust for precision, and ensure non-negative result.
            max(
                (liquidity / price_sqrt - liquidity / price_next_sqrt)
                    .checked_abs()
                    .unwrap()
                    - PreciseDecimal::ATTO,
                pdec!(0),
            )
        }
        SwapType::SellX => {
            // For SellX, directly calculate the output based on liquidity change across price difference.
            liquidity * (price_sqrt - price_next_sqrt).checked_abs().unwrap()
        }
    })
    .floor_to(output_divisibility) // Round down to the nearest output divisibility unit.
}

#[derive(ScryptoSbor, Clone, Debug)]
pub struct SwapState {
    pub pool_address: ComponentAddress,
    pub input_address: ResourceAddress,
    pub output_address: ResourceAddress,
    pub input: Decimal,
    pub input_divisibility: u8,
    pub output: Decimal,
    pub output_divisibility: u8,
    pub remainder: Decimal,
    pub remainder_fee_lp: Decimal,
    pub liquidity: PreciseDecimal,
    pub active_tick: Option<i32>,
    pub price_sqrt: PreciseDecimal,
    pub price_sqrt_sell_cache: Option<PreciseDecimal>,
    pub swap_type: SwapType,
    pub input_fee_rate: Decimal,
    pub fee_protocol_share: Decimal,
    pub fee_lp_share: Decimal,
    pub input_share: Decimal,
    pub fee_lp_input: Decimal,
    pub fee_protocol_input: Decimal,
    pub fee_protocol_max: Decimal,
    pub global_input_fee_lp: PreciseDecimal,
    pub global_output_fee_lp: PreciseDecimal,
    pub global_seconds: u64,
    pub crossed_ticks: Vec<TickOutside>,
}

impl SwapState {
    /// Calculates the new price based on the provided liquidity for the remaining input.
    ///
    /// # Arguments
    /// * `liquidity` - The updated liquidity value.
    ///
    /// # Returns
    /// * `PreciseDecimal` - The newly calculated price.
    pub fn new_price(&self, liquidity: PreciseDecimal) -> PreciseDecimal {
        new_price(
            self.swap_type,
            liquidity,
            self.price_sqrt,
            self.remainder,
            self.input_divisibility,
        )
    }

    /// Checks if the liquidity is zero.
    ///
    /// This method is used to determine if the pool has any active liquidity.
    /// A result of `true` indicates that the current sub-pool has no liquidity and there is a liquidity gap.
    ///
    /// # Returns
    /// * `bool` - True if liquidity is zero, otherwise false.
    pub fn liquidity_is_zero(&self) -> bool {
        self.liquidity == PreciseDecimal::ZERO
    }

    /// Checks if the remainder is zero or negative.
    ///
    /// This method is used to verify if all inputs have been fully utilized in the swap.
    /// A non-positive remainder indicates that there are no leftover inputs.
    ///
    /// # Returns
    /// * `bool` - True if remainder is zero or negative, otherwise false.
    pub fn remainder_is_empty(&self) -> bool {
        self.remainder <= Decimal::ZERO
    }

    /// Determines if the new price does not reach the specified tick price.
    ///
    /// This method is essential for deciding whether a partial or full step swap should be executed,
    /// based on the direction of the swap and the new price relative to the tick price.
    ///
    /// # Arguments
    /// * `tick_price_sqrt` - The square root of the tick's price.
    /// * `price_new_sqrt` - The square root of the new price.
    ///
    /// # Returns
    /// * `bool` - True if the new price does not reach the tick price, otherwise false.
    pub fn not_reaching_tick(
        &self,
        tick_price_sqrt: PreciseDecimal,
        price_new_sqrt: PreciseDecimal,
    ) -> bool {
        match self.swap_type {
            SwapType::BuyX => price_new_sqrt < tick_price_sqrt,
            SwapType::SellX => tick_price_sqrt < price_new_sqrt,
        }
    }

    /// Adjusts the liquidity and updates the active tick index during a tick crossing.
    ///
    /// This method is critical for maintaining accurate state when the price crosses a tick boundary.
    /// It updates the liquidity and the active tick index, ensuring the pool's state is consistent.
    ///
    /// # Arguments
    /// * `crossed_tick` - The tick that has been crossed.
    /// * `next_active_tick_index` - The index of the next active tick.
    pub fn cross_tick(&mut self, crossed_tick: &mut Tick, next_active_tick_index: i32) {
        self.adjust_liquidity(crossed_tick.delta_liquidity);
        self.active_tick = Some(next_active_tick_index);
        let (global_x_fee_lp, global_y_fee_lp) = match self.swap_type {
            SwapType::BuyX => (self.global_output_fee_lp, self.global_input_fee_lp),
            SwapType::SellX => (self.global_input_fee_lp, self.global_output_fee_lp),
        };
        self.crossed_ticks.push(crossed_tick.update_outside_values(
            global_x_fee_lp,
            global_y_fee_lp,
            self.global_seconds,
        ));
    }

    /// Executes a partial step in the swap process when the new price does not reach the next tick.
    ///
    /// This method is used when the price movement is insufficient to reach the next tick.
    /// It updates the output based on the new price and adjusts the remainder and fees accordingly.
    ///
    /// # Arguments
    /// * `price_new_sqrt` - The square root of the new price.
    pub fn partial_step_swap(&mut self, price_new_sqrt: PreciseDecimal) {
        let output = output_step(
            self.swap_type,
            self.liquidity,
            self.price_sqrt,
            price_new_sqrt,
            self.output_divisibility,
        );
        self.output += output;
        self.input += self.remainder;
        self.remainder = dec!(0);

        /*
        remainder_fee_lp has the full remainder, meaning there are no rounding errors
        and the partial swap simply gets the remaining fees
         */
        self.global_input_fee_lp += self.remainder_fee_lp / self.liquidity;
        self.fee_lp_input += self.remainder_fee_lp;
        self.remainder_fee_lp = dec!(0);

        self.price_sqrt = price_new_sqrt;
    }

    /// Takes protocol fees from the liquidity pool's fee reserves.
    ///
    /// This method is invoked to allocate the maximum allowable protocol fees or
    /// calculate partial fees in case of a swap with a remainder.
    pub fn take_protocol_fees(&mut self) {
        if self.remainder_is_empty() {
            self.fee_protocol_input = self.fee_protocol_max;
            return;
        }

        let partial_protocol_fee = (PreciseDecimal::from(self.fee_lp_input) / self.fee_lp_share
            - self.fee_lp_input)
            .ceil_to(self.input_divisibility);
        // Ensure that the calculated and ceiled protocol fees do not exceed the predefined maximum.
        self.fee_protocol_input = min(self.fee_protocol_max, partial_protocol_fee);
    }

    /// Executes a full step in the swap process when the new price reaches the next tick.
    ///
    /// This method is used when the price movement is sufficient to reach or surpass the next tick.
    /// It updates the output and input based on the tick's price and adjusts the remainder and fees accordingly.
    ///
    /// # Arguments
    /// * `tick` - The tick that defines the next price level.
    pub fn full_step_swap(&mut self, tick: &Tick) {
        let output = output_step(
            self.swap_type,
            self.liquidity,
            self.price_sqrt,
            tick.price_sqrt,
            self.output_divisibility,
        );
        self.output += output;
        let step_input = input_step(
            self.swap_type,
            self.liquidity,
            self.price_sqrt,
            tick.price_sqrt,
            self.input_divisibility,
        );
        self.input += step_input;
        self.remainder -= step_input;

        let total_fee_step = PreciseDecimal::from(step_input) / self.input_share - step_input;
        let fee_lp_input_delta =
            (total_fee_step * self.fee_lp_share).floor_to(self.input_divisibility);
        self.global_input_fee_lp += fee_lp_input_delta / self.liquidity;
        self.fee_lp_input += fee_lp_input_delta;
        self.remainder_fee_lp -= fee_lp_input_delta;

        self.price_sqrt = tick.price_sqrt;
    }

    /// Adjusts the pool's liquidity by adding or subtracting the specified amount.
    ///
    /// This method is essential for correctly updating the pool's liquidity during swaps or liquidity changes.
    /// It ensures that the liquidity is adjusted accurately, reflecting the actual state of the pool.
    ///
    /// # Arguments
    /// * `delta_liquidity` - The amount by which to adjust the liquidity.
    pub fn adjust_liquidity(&mut self, delta_liquidity: PreciseDecimal) {
        self.liquidity = new_liquidity(self.swap_type, self.liquidity, delta_liquidity);
    }

    /// Determines the type of swap step (partial or full) based on whether the new price reaches the next tick.
    ///
    /// # Arguments
    /// * `tick` - The tick that defines the next price level.
    /// * `price_new_sqrt` - The square root of the new price.
    ///
    /// # Returns
    /// * `Option<IterMutControl>` - Control flow signal for iterator, `Some(IterMutControl::Break)` if partial step is taken, otherwise `None`.
    pub fn step_swap(
        &mut self,
        tick: &Tick,
        price_new_sqrt: PreciseDecimal,
    ) -> Option<IterMutControl> {
        if self.not_reaching_tick(tick.price_sqrt, price_new_sqrt) {
            self.partial_step_swap(price_new_sqrt);
            return Some(IterMutControl::Break);
        }
        self.full_step_swap(tick);
        None
    }
}

impl From<SwapState> for AfterSwapState {
    fn from(state: SwapState) -> Self {
        AfterSwapState {
            pool_address: state.pool_address,
            input_address: state.input_address,
            output_address: state.output_address,
            price_sqrt: state.price_sqrt,
            active_liquidity: state.liquidity,
            swap_type: state.swap_type,
            input_fee_rate: state.input_fee_rate,
            fee_protocol_share: state.fee_protocol_share,
            input_amount: state.input,
            output_amount: state.output,
            input_fee_lp: state.fee_lp_input,
            input_fee_protocol: state.fee_protocol_input,
        }
    }
}

/// Calculates the effective value within a specified range, adjusting for values outside the range.
///
/// This function is essential for determining the net contribution of a variable (like fees or seconds) within a specified range.
/// It subtracts the contributions outside the specified bounds to provide a precise measure of the variable's effective value within the range.
/// This is particularly useful in scenarios where only the contributions within a certain price or time range are relevant,
/// such as calculating fees accrued within specific price boundaries in a liquidity pool.
///
/// # Type Parameters
/// * `T`: A type that supports addition, subtraction, copying, and display. Typically used for numerical calculations.
///
/// # Arguments
/// * `value_global`: The total value from which contributions outside the specified range will be subtracted.
/// * `value_outside_left`: The value outside the left boundary of the range.
/// * `value_outside_right`: The value outside the right boundary of the range.
/// * `active_tick`: An optional tick index representing the current position. If `None`, it defaults to the minimum possible tick.
/// * `left_bound`: The left boundary of the range.
/// * `right_bound`: The right boundary of the range.
///
/// # Returns
/// * `T`: The net value within the specified range after adjusting for external contributions.
pub fn value_in_range<T: Add<Output = T> + Sub<Output = T> + Copy + Display>(
    value_global: T,
    value_outside_left: T,
    value_outside_right: T,
    active_tick: Option<i32>,
    left_bound: i32,
    right_bound: i32,
) -> T {
    let active_tick = active_tick.unwrap_or(i32::MIN);

    let value_below_left = if active_tick >= left_bound {
        value_outside_left
    } else {
        value_global - value_outside_left
    };

    let value_above_right = if active_tick >= right_bound {
        value_global - value_outside_right
    } else {
        value_outside_right
    };

    value_global - value_below_left - value_above_right
}

/// Executes a buy step in the swap process.
///
/// This function is called during a swap when the swap type is buying. It adjusts the state based on the current tick and the liquidity available.
/// It determines the new price, crosses the tick if necessary, and decides whether to continue or break the iteration based on the remainder.
///
/// # Arguments
/// * `state` - The mutable reference to the current swap state.
/// * `tick` - The mutable reference to the current tick data.
/// * `tick_index` - The index of the current tick.
///
/// # Returns
/// * `IterMutControl` - An enum indicating whether to continue or break the iteration.
pub fn buy_step(state: &mut SwapState, tick: &mut Tick, tick_index: i32) -> IterMutControl {
    if state.liquidity_is_zero() {
        state.price_sqrt = tick.price_sqrt;
        state.cross_tick(tick, tick_index);
        return IterMutControl::Continue;
    }

    let price_new_sqrt = state.new_price(state.liquidity);
    if let Some(control) = state.step_swap(tick, price_new_sqrt) {
        return control;
    }

    state.cross_tick(tick, tick_index);

    if state.remainder_is_empty() {
        return IterMutControl::Break;
    }

    return IterMutControl::Continue;
}

/// Executes a sell step in the swap process.
///
/// This function is called during a swap when the swap type is selling. It adjusts the state based on the current and next ticks, and the liquidity available.
/// It determines the new price, potentially caches it for future use, and decides whether to continue or break the iteration based on the remainder and next tick index.
///
/// # Arguments
/// * `state` - The mutable reference to the current swap state.
/// * `tick` - The mutable reference to the current tick data.
/// * `next_tick_index` - The optional index of the next tick.
///
/// # Returns
/// * `IterMutControl` - An enum indicating whether to continue or break the iteration.
pub fn sell_step(
    state: &mut SwapState,
    tick: &mut Tick,
    next_tick_index: Option<i32>,
) -> IterMutControl {
    if state.liquidity == pdec!(0) {
        state.price_sqrt = tick.price_sqrt;
    }

    if state.price_sqrt != tick.price_sqrt {
        let price_new_sqrt = state
            .price_sqrt_sell_cache
            .take()
            .unwrap_or_else(|| state.new_price(state.liquidity));
        if let Some(control) = state.step_swap(tick, price_new_sqrt) {
            return control;
        }
    }

    // crossing tick if there is liquidity and the remainder is enough to move the price
    if state.remainder_is_empty() || next_tick_index.is_none() {
        return IterMutControl::Break;
    }

    let next_liquidity = new_liquidity(state.swap_type, state.liquidity, tick.delta_liquidity);
    if next_liquidity.is_zero() {
        state.cross_tick(tick, next_tick_index.unwrap());
        return IterMutControl::Continue;
    }

    let price_new_sqrt = state.new_price(next_liquidity);
    if price_new_sqrt == state.price_sqrt {
        return IterMutControl::Break;
    }

    state.cross_tick(tick, next_tick_index.unwrap());
    state.price_sqrt_sell_cache = Some(price_new_sqrt);

    IterMutControl::Continue
}

#[cfg(test)]
mod test {
    use scrypto::prelude::*;

    use super::*;

    #[test]
    fn test_new_price() {
        assert_eq!(
            new_price(SwapType::SellX, pdec!(10), pdec!(1), dec!(0), 18),
            pdec!(1)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{max_liquidity_per_tick, number_of_ticks};
    use crate::constants::*;
    use pretty_assertions::assert_eq;
    use scrypto::prelude::*;
    use test_case::test_case;

    #[test_case(MAX_TICK, 3)]
    #[test_case(MAX_TICK - 1, 3)]
    #[test_case(MAX_TICK / 2 + 1, 3)]
    #[test_case(MAX_TICK / 2, 5)]
    #[test_case(1, MAX_TICK * 2 + 1)]
    fn test_number_of_ticks(spacing: i32, result: i32) {
        assert_eq!(number_of_ticks(spacing as u32), result as u32)
    }

    #[test_case(MAX_TICK, MAX_LIQUIDITY / 3)] // 3 ticks
    #[test_case(MAX_TICK - 1, MAX_LIQUIDITY / 3)] // 3 ticks
    #[test_case(MAX_TICK / 2, MAX_LIQUIDITY / 5)] // 5 ticks
    #[test_case(1, MAX_LIQUIDITY / (2 * MAX_TICK + 1))] // 5 ticks
    fn test_max_liquidity_per_tick(spacing: i32, result: PreciseDecimal) {
        assert_eq!(max_liquidity_per_tick(spacing as u32), result)
    }
}
