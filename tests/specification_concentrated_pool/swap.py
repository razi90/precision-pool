import logging
from collections import defaultdict
from typing import DefaultDict, List, Tuple
from decimal import Decimal
import decimal
from add_liquidity import add_liquidity

from pool import *
from constants import *
from scryptomath import PreciseDecimal, floor_to_decimal, ceil_to_decimal, Decimal as SDecimal

decimal.getcontext().prec = 500


def calculate_ticks(positions):
    ticks = defaultdict(Decimal)
    positions = [position for position in positions if position is not None]
    for liquidity, left_bound, right_bound in positions:
        ticks[left_bound] += liquidity
        ticks[right_bound] -= liquidity
    return ticks

def get_left_ticks(ticks, price_sqrt: Decimal):
    return sorted([tick for tick in ticks if tick_to_price(tick).sqrt() <= price_sqrt], reverse=True)

def get_right_ticks(ticks, price_sqrt: Decimal):
    return sorted([tick for tick in ticks if price_sqrt < tick_to_price(tick).sqrt()])

def calc_active_liquidity(ticks, price_sqrt: Decimal, input_is_x: bool):
    if input_is_x:
        return sum(liquidity_delta for tick, liquidity_delta in ticks.items() if tick_to_price(tick).sqrt() < price_sqrt)
    return sum(liquidity_delta for tick, liquidity_delta in ticks.items() if tick_to_price(tick).sqrt() <= price_sqrt)

def calc_new_price_sqrt(
    active_liquidity: Decimal,
    price_sqrt: Decimal,
    input_amount: Decimal,
    input_is_x: bool
):
    # smaller price movement corresponds to more input tokens than required
    if input_is_x:
        return (active_liquidity * price_sqrt) / (active_liquidity + input_amount * price_sqrt)
    return input_amount / active_liquidity + price_sqrt

def x_delta(active_liquidity: Decimal, price_a_sqrt: Decimal, price_b_sqrt: Decimal):
    return abs(active_liquidity / price_a_sqrt - active_liquidity / price_b_sqrt)

def y_delta(active_liquidity: Decimal, price_a_sqrt: Decimal, price_b_sqrt: Decimal):
    return active_liquidity * abs(price_a_sqrt - price_b_sqrt)

def input_in_step(active_liquidity: Decimal, price_a_sqrt: Decimal, price_b_sqrt: Decimal, input_is_x: bool):
    delta = x_delta if input_is_x else y_delta
    return ceil_to_decimal(delta(active_liquidity, price_a_sqrt, price_b_sqrt))

def output_in_step(active_liquidity: Decimal, price_a_sqrt: Decimal, price_b_sqrt: Decimal, input_is_x: bool):
    delta = y_delta if input_is_x else x_delta
    return floor_to_decimal(delta(active_liquidity, price_a_sqrt, price_b_sqrt))

def log_price(price_sqrt: Decimal, new_line: bool = False):
    logging.debug(f"price_sqrt = {PreciseDecimal(price_sqrt):f}")
    logging.debug(f"price = {PreciseDecimal(price_sqrt ** 2):f}" + ("\n" if new_line else ""))

def calculate_fee(amount: Decimal, fee_rate: Decimal, fee_protocol_rate: Decimal):
    fee_total = amount * fee_rate
    fee_lp = fee_total * (1 - fee_protocol_rate)
    fee_protocol = fee_total * fee_protocol_rate
    amount_net = amount * (1 - fee_rate)

    return amount_net, fee_lp, fee_protocol

def position_is_active(position, price_a_sqrt: Decimal, price_b_sqrt: Decimal):
    liquidity, left_bound, right_bound = position
    price_left_bound_sqrt = tick_to_price_sqrt(left_bound)
    price_right_bound_sqrt = tick_to_price_sqrt(right_bound)
    price_mid_sqrt = (price_a_sqrt + price_b_sqrt) / 2
    return price_left_bound_sqrt <= price_mid_sqrt <= price_right_bound_sqrt

def add_dicts(dict1, dict2):
    result = dict1.copy()  # Make a copy of the first dictionary to preserve its original contents
    for key, value in dict2.items():
        if key in result:
            result[key] += value
        else:
            result[key] = value
    return result

class Fees:
    def __init__(self, fee_prot_x = None, fee_prot_y = None, fee_lp_x = None, fee_lp_y = None) -> None:
        self.fee_prot_x = fee_prot_x or Decimal(0)
        self.fee_prot_y = fee_prot_y or Decimal(0)
        self.fee_lp_x = fee_lp_x or defaultdict(decimal.Decimal)
        self.fee_lp_y = fee_lp_y or defaultdict(decimal.Decimal)

    def add_fees(self, positions, price_a_sqrt: Decimal, price_b_sqrt: Decimal, prot_in: Decimal, lp_in: Decimal, input_is_x: bool):
        active_positions = {
            i: position[0]
            for i, position in enumerate(positions)
            if position_is_active(position, price_a_sqrt, price_b_sqrt)
        }
        total_liquidity = sum(active_positions.values())
        self.fee_prot_x += prot_in if input_is_x else Decimal(0)
        self.fee_prot_y += Decimal(0) if input_is_x else prot_in
        lp_x = lp_in if input_is_x else Decimal(0)
        lp_y = Decimal(0) if input_is_x else lp_in
        self.fee_lp_x = add_dicts(self.fee_lp_x, {
            i: liquidity / total_liquidity * lp_x
            for i, liquidity in active_positions.items()
        })
        self.fee_lp_y = add_dicts(self.fee_lp_y, {
            i: liquidity / total_liquidity * lp_y
            for i, liquidity in active_positions.items()
        })

    def claim_fee_lp(self, position_id: int):
        fee_lp_x = self.fee_lp_x[position_id]
        fee_lp_y = self.fee_lp_y[position_id]
        logging.info(f"CLAIM FEES - Position ID {position_id}: {SDecimal(fee_lp_x):f} X, {SDecimal(fee_lp_y):f} Y\n")
        self.fee_lp_x[position_id] = Decimal(0)
        self.fee_lp_y[position_id] = Decimal(0)
        return fee_lp_x, fee_lp_y

    def __repr__(self):
        position_ids = set(self.fee_lp_x) | set(self.fee_lp_y)
        lps = "\n".join(f"  {i}: {SDecimal(self.fee_lp_x[i]):f} X, {SDecimal(self.fee_lp_y[i]):f} Y" for i in position_ids)
        return f"Prot: {SDecimal(self.fee_prot_x):f} X, {SDecimal(self.fee_prot_y):f} Y, LP: {SDecimal(sum(self.fee_lp_x.values())):f} X, {SDecimal(sum(self.fee_lp_y.values())):f}  Y\nLPs:\n{lps}"

    def __add__(self, other):
        if not isinstance(other, Fees):
            raise TypeError("Unsupported operand type")
        return Fees(
            fee_prot_x = self.fee_prot_x + other.fee_prot_x,
            fee_prot_y = self.fee_prot_y + other.fee_prot_y,
            fee_lp_x = add_dicts(self.fee_lp_x, other.fee_lp_x),
            fee_lp_y = add_dicts(self.fee_lp_y, other.fee_lp_y)
        )


def calculate_swap(
        positions: List,
        price_sqrt: Decimal,
        input_amount: Decimal,
        input_is_x: bool,
        fee_input_rate: Decimal = None,
        fee_protocol_rate: Decimal = None
    ):
    fee_input_rate = fee_input_rate or Decimal(0)
    fee_protocol_rate = fee_protocol_rate or Decimal(0)
    fees = Fees()
    symbol = "X" if input_is_x else "Y"
    logging.info(f"SWAP:  Price = {PreciseDecimal(price_sqrt**2):f}, Input = {input_amount:f} {symbol}, Positions = {positions}")
    ticks = calculate_ticks(positions)
    output = Decimal(0)
    remaining_input = input_amount
    next_ticks = get_left_ticks(ticks, price_sqrt) if input_is_x else get_right_ticks(ticks, price_sqrt)
    active_liquidity = 0
    logging.debug(next_ticks)
    for next_tick in next_ticks:
        log_price(price_sqrt)
        logging.debug(f"next_tick = {next_tick}")
        price_next_tick_sqrt = tick_to_price(next_tick).sqrt()
        active_liquidity = calc_active_liquidity(ticks, price_sqrt, input_is_x)
        logging.debug(f"active_liquidity = {active_liquidity:f}")

        if active_liquidity == 0:
            logging.debug("move to next_tick")
            price_sqrt = price_next_tick_sqrt
            continue

        remaining_input_net, fee_input_lp, fee_input_prot = calculate_fee(remaining_input, fee_input_rate, fee_protocol_rate)
        price_new_sqrt = calc_new_price_sqrt(active_liquidity, price_sqrt, remaining_input_net, input_is_x)

        # partially consume current sub pool
        if (input_is_x and price_next_tick_sqrt < price_new_sqrt) or (not input_is_x and price_new_sqrt < price_next_tick_sqrt):
            logging.debug("Partial swap step")
            output_step = output_in_step(active_liquidity, price_sqrt, price_new_sqrt, input_is_x)
            fees.add_fees(positions, price_sqrt, price_new_sqrt, fee_input_prot, fee_input_lp, input_is_x)
            logging.debug(f"fee_input_lp = {SDecimal(fee_input_lp):f}, fee_input_prot = {SDecimal(fee_input_prot):f}")
            output += output_step
            remaining_input = Decimal(0)
            price_sqrt = price_new_sqrt
            break

        # fully consume current sub pool
        logging.debug("Full swap step")
        output_step = output_in_step(active_liquidity, price_sqrt, price_next_tick_sqrt, input_is_x)
        output += output_step

        input_step = input_in_step(active_liquidity, price_sqrt, price_next_tick_sqrt, input_is_x)
        input_step_with_fees = input_step / (Decimal(1) - fee_input_rate)
        _, fee_input_lp, fee_input_prot = calculate_fee(input_step_with_fees, fee_input_rate, fee_protocol_rate)
        fees.add_fees(positions, price_sqrt, price_next_tick_sqrt, fee_input_prot, fee_input_lp, input_is_x)
        remaining_input -= input_step_with_fees
        logging.debug(f"input_step = {SDecimal(input_step):f}, fee_input_lp = {SDecimal(fee_input_lp):f}, fee_input_prot = {SDecimal(fee_input_prot):f}")
        price_sqrt = price_next_tick_sqrt

        if remaining_input <= 0:
            break

    logging.info(f"Output = {output:f}")
    logging.info(f"Remainder  = {remaining_input:f}")
    log_price(price_sqrt, new_line=True)
    logging.info(f"Fees: {fees}")
    return price_sqrt, output, remaining_input, fees

def swap_success(price: Decimal, positions, trades: List[Tuple[Decimal, bool]]) -> Decimal:
    price_sqrt = price.sqrt()
    added_positions = []
    for (left_bound, right_bound, x_amount, y_amount) in positions:
        L, _, _ = add_liquidity(price, left_bound, right_bound, x_amount, y_amount)
        added_positions.append((L, left_bound, right_bound))
    for amount, input_is_x in trades:
        price_sqrt, output, remainder, _ = calculate_swap(added_positions, price_sqrt, amount, input_is_x)
    return price_sqrt, output, remainder


def swap_buy_success(price: Decimal, positions, amount: Decimal) -> Decimal:
    return swap_success(price, positions, [(amount, False)])

def swap_sell_success(price: Decimal, positions, amount: Decimal) -> Decimal:
    return swap_success(price, positions, [(amount, True)])