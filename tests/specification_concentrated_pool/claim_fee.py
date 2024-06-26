from collections import defaultdict
import logging
from decimal import Decimal
from typing import List, Tuple
from add_liquidity import add_liquidity
from swap import calculate_swap, Fees
from remove_liquidity import remove_liquidity
from scryptomath import Decimal as SDecimal, PreciseDecimal

# ADD 1, ADD 2, SWAP, CLAIM 2, REMOVE 2
# ADD, SWAP, CLAIM, SWAP, REMOVE (fees in swap and claim, no fees anymore in remove)

def claim_fee_success(
        price_sqrt: Decimal,
        positions_before,
        trades_before: List[Tuple[Decimal, bool]] = None,
        claim_ids: List[int] = None,
        trades_after: List[Tuple[Decimal, bool]] = None,
        remove_ids_after: List[int] = None,
        fee_input_rate: Decimal = None,
        fee_protocol_rate: Decimal = None
    ):
    trades_before = trades_before or []
    claim_ids = claim_ids or []
    trades_after = trades_after or []
    positions = []
    trades_before_output = []
    claim_output = []
    trades_after_output = []
    remove_after_output = []
    fees = Fees()

    for (left_bound, right_bound, x_amount, y_amount) in positions_before:
        L, _, _ = add_liquidity(None, left_bound, right_bound, x_amount, y_amount, price_sqrt=price_sqrt)
        positions.append((L, left_bound, right_bound))
    for amount, input_is_x in trades_before:
        price_sqrt, output, remainder, trade_fees  = calculate_swap(positions, price_sqrt, amount, input_is_x, fee_input_rate=fee_input_rate, fee_protocol_rate=fee_protocol_rate)
        fees += trade_fees
        trades_after_output.append((price_sqrt, output, remainder))
    for claim_id in claim_ids:
        claim_output.append(fees.claim_fee_lp(claim_id))
    for amount, input_is_x in trades_after:
        price_sqrt, output, remainder, trade_fees = calculate_swap(positions, price_sqrt, amount, input_is_x, fee_input_rate=fee_input_rate, fee_protocol_rate=fee_protocol_rate)
        fees += trade_fees
        trades_after_output.append((price_sqrt, output, remainder))
    for remove_id in remove_ids_after or []:
        remove_L, remove_left_tick, remove_right_tick = positions[remove_id]
        x_fee, y_fee = fees.claim_fee_lp(remove_id)
        x_returned, y_returned = remove_liquidity(remove_L, price_sqrt, remove_left_tick, remove_right_tick, x_fee, y_fee)
        remove_after_output.append((x_returned, y_returned))
    return price_sqrt, trades_before_output, trades_after_output, claim_output, remove_after_output

def claim_fee_default_scenario(price_sqrt, positions, input_is_x: bool, fee_rate: Decimal, amount=1):
    trade_before = [(Decimal(amount), input_is_x)]
    trade_after = [(Decimal(amount), not input_is_x)]
    fee_rates = {
        "fee_input_rate": fee_rate,
        "fee_protocol_rate": fee_rate
    }
    logging.info("\n\n########\nCLAIM FIRST - REMOVE FIRST\n########\n")
    claim_first_remove_first = claim_fee_success(price_sqrt, positions, trade_before, [0], trade_after, [0], **fee_rates)
    logging.info("\n\n########\nCLAIM FIRST - REMOVE SECOND\n########\n")
    claim_first_remove_second = claim_fee_success(price_sqrt, positions, trade_before, [0], trade_after, [1], **fee_rates)
    logging.info("\n\n########\nCLAIM SECOND - REMOVE FIRST\n########\n")
    claim_second_remove_first = claim_fee_success(price_sqrt, positions, trade_before, [1], trade_after, [0], **fee_rates)
    logging.info("\n\n########\nCLAIM SECOND - REMOVE SECOND\n########\n")
    claim_second_remove_second = claim_fee_success(price_sqrt, positions, trade_before, [1], trade_after, [1], **fee_rates)

    def log_scenario(name, price_sqrt, trades_before_output, trades_after_output, claim_output, remove_after_output):
        logging.info(name)
        logging.info(f"price = {PreciseDecimal(price_sqrt ** 2)}")
        for x_claimed, y_claimed in claim_output:
            logging.info(f"CLAIM: {SDecimal(x_claimed):f} X, {SDecimal(y_claimed):f} X")
        for x_returned, y_returned in remove_after_output:
            logging.info(f"REMOVE: ({x_returned:F} X, {y_returned:f} Y)\n")

    log_scenario("CLAIM FIRST - REMOVE FIRST", *claim_first_remove_first)
    log_scenario("CLAIM FIRST - REMOVE SECOND", *claim_first_remove_second)
    log_scenario("CLAIM SECOND - REMOVE FIRST", *claim_second_remove_first)
    log_scenario("CLAIM SECOND - REMOVE SECOND",  *claim_second_remove_second)


def claim_fee_default_scenario_buy(price_sqrt, positions, fee_rate: Decimal, amount=1):
    return claim_fee_default_scenario(price_sqrt, positions, False, fee_rate, amount)

def claim_fee_default_scenario_sell(price_sqrt, positions, fee_rate: Decimal, amount=1):
    return claim_fee_default_scenario(price_sqrt, positions, True, fee_rate, amount)

