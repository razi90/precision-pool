from decimal import Decimal
import decimal
import logging

from pool import *
from constants import *
from scryptomath import PreciseDecimal, floor_to_decimal, ceil_to_decimal

decimal.getcontext().prec = 500

def calc_amounts_for_liquidity(L, price_sqrt, i_c, i_l, i_r):
    price_left_sqrt = tick_to_price_sqrt(i_l)
    price_right_sqrt = tick_to_price_sqrt(i_r)
    if i_c < i_l:
        Y = 0
        X = L * (1/price_left_sqrt - 1/price_right_sqrt)
    elif i_l <= i_c < i_r:
        Y = L * (price_sqrt - price_left_sqrt)
        X = L * (1/price_sqrt - 1/price_right_sqrt)
    elif i_c >= i_r:
        Y = L * (price_right_sqrt - price_left_sqrt)
        X = 0
    else:
        ValueError("This case doesn't exist.")
    return (X, Y)

def calc_liquidities_from_amounts(X, Y, price_sqrt, i_c, i_l, i_r):
    # the larger X, Y -> the larger LX, LY
    # the smaller the (absolute price) range -> the larger LX, LY
    price_left_sqrt = tick_to_price_sqrt(i_l)
    price_right_sqrt = tick_to_price_sqrt(i_r)
    logging.debug(f"price_sqrt={PreciseDecimal(price_sqrt):f}")
    logging.debug(f"price_left_sqrt={PreciseDecimal(price_left_sqrt):f}")
    logging.debug(f"price_right_sqrt={PreciseDecimal(price_right_sqrt):f}")
    if i_c < i_l:
        LY = None
        LX = X / (1/price_left_sqrt - 1/price_right_sqrt)
    elif i_l <= i_c < i_r:
        LY = Y / (price_sqrt - price_left_sqrt) if price_sqrt != price_left_sqrt else None
        LX = X / (1/price_sqrt - 1/price_right_sqrt) if price_sqrt != price_right_sqrt else None
    elif i_c >= i_r:
        LY = Y / (price_right_sqrt - price_left_sqrt)
        LX = None
    else:
        ValueError("This case doesn't exist.")
    return (LX, LY)

def calc_allowed_inputs(X, Y, i_c, i_l, i_r, price_sqrt):
    # reduce amounts by small fraction to account for ceiling at the end
    X_safe = max(X - Decimal("0.000000000000000002"), Decimal(0))
    Y_safe = max(Y - Decimal("0.000000000000000002"), Decimal(0))

    # Calc equiv. liquidities
    LX, LY = calc_liquidities_from_amounts(X_safe, Y_safe, price_sqrt, i_c, i_l, i_r)

    # Check allowed liquidity
    if LX == None:
        L = LY
    elif LY == None:
        L = LX
    else:
        L = min(LX, LY)

    L = floor_to_decimal(L, precision=64)
    L = min(L, MAX_LIQUIDITY_PER_TICK)

    # Calc allowed inputs
    X_allowed, Y_allowed = calc_amounts_for_liquidity(L, price_sqrt, i_c, i_l, i_r)
    X_allowed = ceil_to_decimal(X_allowed + ATTO_DECIMAL)
    Y_allowed = ceil_to_decimal(Y_allowed + ATTO_DECIMAL)

    # Dust handling
    X_allowed = X if X - X_allowed <= Decimal("0.000000000000000002") else X_allowed
    Y_allowed = Y if Y - Y_allowed <= Decimal("0.000000000000000002") else Y_allowed

    return (L, X_allowed, Y_allowed)

def calc_allowed_inputs_wrapper(X, Y, i_l, i_r, price_sqrt):
    p_i_l = tick_to_price_sqrt(i_l)
    p_i_r = tick_to_price_sqrt(i_r)

    if price_sqrt < p_i_l:
        i_c = i_l - 100
    elif p_i_l <= price_sqrt < p_i_r:
        i_c = i_l
    elif p_i_r <= price_sqrt:
        i_c = i_r

    return calc_allowed_inputs(X, Y, i_c, i_l, i_r, price_sqrt)

def add_liquidity(price, left_bound, right_bound, x_amount, y_amount, price_sqrt=None):
    if price_sqrt is None:
        price_sqrt = price.sqrt()
    L, x_allowed, y_allowed = calc_allowed_inputs_wrapper(x_amount, y_amount, left_bound, right_bound, price_sqrt)
    x_returned, y_returned = (x_amount - x_allowed, y_amount - y_allowed)
    logging.info(f"ADD LIQUIDITY: price_sqrt={price_sqrt:f} LB {left_bound} RB {right_bound} X {x_amount:f} Y {y_amount:f}")
    logging.info(f"L {L:f}")
    logging.info(f"Allowed ({x_allowed:f}, {y_allowed:f})")
    logging.info(f"Returned ({x_returned:F}, {y_returned:f})\n")
    return L, x_allowed, y_allowed


def add_liquidity_position(price, left_bound, right_bound, x_amount, y_amount):
    L, _, _ = add_liquidity(price, left_bound, right_bound, x_amount, y_amount)
    return L, left_bound, right_bound
