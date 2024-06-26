import logging
import decimal
from typing import List, Tuple
from add_liquidity import add_liquidity
from swap import calculate_swap
from pool import *
from constants import *
from scryptomath import floor_to_decimal, Decimal as SDecimal, PreciseDecimal
from add_liquidity import calc_amounts_for_liquidity

decimal.getcontext().prec = 500


def removed_amounts_inputs_wrapper(L, price_sqrt, i_l, i_r):
    p_i_l = tick_to_price_sqrt(i_l)
    p_i_r = tick_to_price_sqrt(i_r)

    if price_sqrt < p_i_l:
        i_c = i_l - 100
    elif p_i_l <= price_sqrt < p_i_r:
        i_c = i_l
    elif p_i_r <= price_sqrt:
        i_c = i_r

    return calc_amounts_for_liquidity(L, price_sqrt, i_c, i_l, i_r)

def remove_liquidity(liquidity, price_sqrt, left_bound, right_bound, x_fee=0, y_fee=0):
    x_returned, y_returned = removed_amounts_inputs_wrapper(liquidity, price_sqrt, left_bound, right_bound)
    x_returned = floor_to_decimal(x_returned + x_fee)
    y_returned = floor_to_decimal(y_returned + y_fee)
    logging.info(f"REMOVE LIQUIDITY: L {liquidity:f} price_sqrt={PreciseDecimal(price_sqrt):f} LB {left_bound} RB {right_bound}")
    logging.info(f"Returned ({x_returned:F}, {y_returned:f}), Fees ({SDecimal(x_fee):F}, {SDecimal(y_fee):f})\n")
    return x_returned, y_returned

def remove_liquidity_success(price: Decimal, positions, trades_before: List[Tuple[Decimal, bool]], remove_ids: List[int], trades_after: List[Tuple[Decimal, bool]], positions_after=None, remove_ids_after: List[int] = None):
    price_sqrt = price.sqrt()
    added_positions = []
    trades_before_output = []
    trades_after_output = []
    remove_output = []
    remove_output_after = []
    for (left_bound, right_bound, x_amount, y_amount) in positions:
        L, _, _ = add_liquidity(price, left_bound, right_bound, x_amount, y_amount)
        added_positions.append((L, left_bound, right_bound))
    for amount, input_is_x in trades_before:
        price_sqrt, output, remainder, _ = calculate_swap(added_positions, price_sqrt, amount, input_is_x)
        trades_after_output.append((price_sqrt, output, remainder))
    for remove_id in remove_ids:
        remove_L, remove_left_tick, remove_right_tick = added_positions.pop(remove_id)
        remove_output.append(remove_liquidity(remove_L, price_sqrt, remove_left_tick, remove_right_tick))
    for (left_bound, right_bound, x_amount, y_amount) in positions_after or []:
        L, _, _ = add_liquidity(price, left_bound, right_bound, x_amount, y_amount)
        added_positions.append((L, left_bound, right_bound))
    for amount, input_is_x in trades_after:
        price_sqrt, output, remainder, _ = calculate_swap(added_positions, price_sqrt, amount, input_is_x)
        trades_after_output.append((price_sqrt, output, remainder))
    for remove_id in remove_ids_after or []:
        remove_L, remove_left_tick, remove_right_tick = added_positions.pop(remove_id)
        remove_output_after.append(remove_liquidity(remove_L, price_sqrt, remove_left_tick, remove_right_tick))
    return price_sqrt, trades_before_output, trades_after_output, remove_output, remove_output_after


def remove_liquidity_success_default(price: Decimal, positions, first_remove_id: int, trades_after: List[Tuple[Decimal, bool]]):
    remove_ids_after = [0 for _ in positions[:-1]]
    return remove_liquidity_success(price, positions, [], remove_ids=[first_remove_id], trades_after=trades_after, remove_ids_after=remove_ids_after)

def remove_liquidity_default_scenario(price, positions, amount=1):
    logging.info("\n\n########\nREMOVE FIRST LP - BUY\n########\n")
    remove_first_buy = remove_liquidity_success_default(price, positions, 0, trades_after=[(Decimal(amount), False)])
    logging.info("\n\n########\nREMOVE SECOND LP - BUY\n########\n")
    remove_second_buy = remove_liquidity_success_default(price, positions, 1, trades_after=[(Decimal(amount), False)])
    logging.info("\n\n########\nREMOVE FIRST LP - SELL\n########\n")
    remove_first_sell = remove_liquidity_success_default(price, positions, 0, trades_after=[(Decimal(amount), True)])
    logging.info("\n\n########\nREMOVE SECOND LP - SELL\n########\n")
    remove_second_sell = remove_liquidity_success_default(price, positions, 1, trades_after=[(Decimal(amount), True)])

    def log_scenario(name, input_is_x: bool, price_sqrt, trades_before_output, trades_after_output, remove_output, remove_output_after):
        logging.info(name)
        logging.info(f"price = {price_sqrt ** 2}")
        # for x_returned, y_returned in remove_output:
        #     logging.info(f"REMOVE: ({x_returned:F} X, {y_returned:f} Y)")
        for _, output, remainder in trades_after_output:
            logging.info(f"SWAP: Output = {output:f}, Remainder  = {remainder:f}")
        for x_returned, y_returned in remove_output_after:
            logging.info(f"REMOVE: ({x_returned:F} X, {y_returned:f} Y)\n")

    log_scenario("REMOVE FIRST LP - BUY", False, *remove_first_buy)
    log_scenario("REMOVE SECOND LP - BUY", False, *remove_second_buy)
    log_scenario("REMOVE FIRST LP - SELL", True, *remove_first_sell)
    log_scenario("REMOVE SECOND LP - SELL", True, *remove_second_sell)