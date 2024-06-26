from decimal import Decimal

from scryptomath import PreciseDecimal
from constants import LOG_10001, TICK_BASE, TICK_BASE_SQRT

def tick_to_price(tick: int):
    return Decimal(PreciseDecimal(TICK_BASE).powi(tick))

def tick_to_price_sqrt(tick: int):
    return Decimal(PreciseDecimal(TICK_BASE_SQRT).powi(tick))

def price_to_tick(price: Decimal):
    return int(price.ln() / LOG_10001)
