# Precision Pool Blueprint

## Overview

The Precision Pool blueprint is designed for managing a liquidity pool with concentrated liquidity features in a decentralized finance (DeFi) environment. This blueprint handles liquidity provision, swaps, fee management, and interactions with external systems like oracles and custom hooks. It supports advanced features like flash loans and integrates with a global registry for protocol fee updates, ensuring the pool operates with the most recent protocol configuration.

## Key Components

### Liquidity Management

- **Add Liquidity**: Liquidity providers can add liquidity within specified price bounds. The blueprint calculates the maximum amount of tokens that can be added, updating the pool's liquidity and tick data to ensure that liquidity is added only within the desired price range, optimizing capital efficiency.

- **Remove Liquidity**: Allows for the withdrawal of tokens by calculating the withdrawable amounts based on the liquidity and price bounds. The blueprint updates the pool's liquidity and adjusts tick data, ensuring that the liquidity is removed correctly and providers receive the appropriate amount of tokens back.

#### Tick System

The pool utilizes a tick-based system to manage liquidity within discrete price intervals. Each tick represents a specific price point, and liquidity providers can choose to provide liquidity within certain price ranges (between two ticks). This system allows for concentrated liquidity, meaning liquidity providers can allocate their assets more efficiently by targeting specific price ranges where they anticipate more trading activity.

#### Tick Alignment and Price Bounds

Ticks are aligned to a specific tick spacing to avoid crossing many too granular ticks during a swap. Each tick corresponds to a potential change in the price due to trading activity. The alignment ensures that the liquidity can be efficiently utilised.

#### Liquidity Calculations

Liquidity in the pool is represented by tokens x and y. The amount of liquidity that can be added or removed is dependent on the current pool price and the bounds set by the liquidity provider. The calculations adjust for token divisibility and ensure that the operations respect the minimum unit of each token, preventing rounding errors and potential imbalances in the pool's state.

#### Price and Liquidity Formulas

The core mathematical model of the pool revolves around the relationship between price and liquidity. The price is modeled as the square root of the ratio of token x to token y, which simplifies the computation of price changes during swaps. Liquidity is calculated based on the current pool price and left and right bounds of liquidity position.

The `price` is given with tick index `i` as:
```
price = 1.0001^i
```

Between every two ticks next to each other there spans a virtual sub pool following the classic AMM liquidity formula:
```
L = sqrt(x * y)
```

With the `price_sqrt`:
```
price_sqrt = sqrt(y/x)
```

Resulting in the virtual reserves `X` and `Y` of the sub pool:
```
X = L / price_sqrt
Y = L * price_sqrt
```

These can be now used to derive the formula for the change in the virtual reserves `x_delta` and `y_delta`:
```
x_delta = L * |1 / price_sqrt - 1 / new_price_sqrt|
y_delta = L * |price_sqrt - new_price_sqrt|
```

The formulas for `x_delta` and `y_delta` serve as the core mathematical basis for all liquidity and swap operations within the pool. These formulas are adapted for various scenarios, including adding or removing liquidity, calculating new prices and determining input and output amounts during swaps.

### Swap Execution

Swaps within the pool influence its pricing, which is denoted by the square root of the ratio between the prices of tokens x and y. The nature of the swap, whether it is a purchase (BuyX) or a sale (SellX), dictates distinct price adjustments. The blueprint manages these transactions by updating the pool's state, which includes both liquidity levels and pricing, to accurately reflect each trade. To maintain market stability and ensure equitable trading conditions, the blueprint offers concentrated liquidity to minimize price fluctuations.

#### Swap Price Calculation

During a swap, the new price is calculated based on the amount of the input token and the current liquidity. The formula ensures that the price adjustment is proportional to the size of the swap relative to the total liquidity, preventing large price swings and ensuring market stability.

#### Swap Steps

Swaps are processed in steps, each corresponding to moving from one tick to the next. The swap might not always reach the next tick if the input amount isn't sufficient to push the price to the next interval. In such cases, a partial step is executed, and the remainder of the tokens is either returned to the user or kept for the next transaction, depending on the specific conditions at the time of the swap.

### Fee Handling

- **Fee Calculation**: Each swap incurs fees, which are divided between the liquidity providers and the protocol treasury. Fees are calculated as a percentage of the swap amount. The division of fees is determined by predefined rates, and the calculations ensure that the fees are distributed accurately according to the stake each party has in the pool. This system incentivizes liquidity provision and protocol maintenance, ensuring long-term sustainability by compensating both parties for their roles in maintaining and utilizing the pool.
- **Registry Synchronization**: The pool synchronizes with a global registry to update protocol fee settings. This ensures that the pool operates with the most recent fee configuration, aligning with broader protocol governance decisions.
- **Protocol Fees**: Allocates a portion of the swap fees to the protocol, supporting the operational sustainability of the pool. The protocol fees are sent to the registry periodically at synchronisation.

### Hooks Integration

Supports custom logic execution through hooks at various stages of pool operations, such as before and after swaps or liquidity changes. This extensibility enables integration with other components or protocols, custom fee logic, or additional security checks, making the pool adaptable to various needs.

### Oracle

Offers a time-weighted price oracle that serves external components by providing precise and timely market data, which is calculated based on the prices from executed swaps within the pool. This functionality is crucial for third-party decentralized applications (dapps) that depend on accurate market pricing.

### Flash Loans

Offers flash loan functionality, allowing users to borrow tokens from the pool within a single transaction, provided they pay back the loan with fees by the end of the transaction. This feature is useful for arbitrage, collateral swapping, or other financial activities that require temporary liquidity and is implemented with strict checks to prevent misuse or risks to the pool's liquidity.

## Detailed Functionality

### Instantiation

- **Blueprint Setup**: Upon creation, the blueprint initializes its state, including setting up vaults for token storage, defining tick spacing for price ranges, and preparing hooks and badges for future operations. This setup phase is critical for ensuring that the pool operates correctly and securely from the start.

### Security Considerations

- **Assertions and Validations**: Throughout the blueprint, various assertions and validations ensure that operations do not proceed under invalid conditions, such as incorrect token amounts or price calculations.

- **Maximum Fee Cap**: To promote fairness and sustain user confidence, the blueprint imposes a fee cap on charges levied through the protocol and its custom hooks. Specifically, hooks are limited to a maximum fee of 10% from users, which aids in preventing prohibitively high fees that could discourage user participation in the pool. The blueprint incorporates robust mechanisms to enforce this fee cap during the execution of hook-related functions. Should hooks attempt to exceed this limit, the transaction is automatically aborted. Additionally, the protocol itself is constrained to a maximum of 25% of the total trading fee.

- **Input Validations**: All inputs to the blueprint methods are rigorously validated. This includes checking token addresses, amounts, price bounds, and fee rates. These checks prevent erroneous or malicious inputs that could disrupt the pool's operations.

## Conclusion

The Precision Pool blueprint is a sophisticated tool for managing a liquidity pool with advanced features like concentrated liquidity, custom hooks, and flash loans. The concentrated liquidity pool's design and implementation leverage complex mathematical models to ensure efficient market operations. Its design focuses on security, efficiency, and extensibility, making it a robust solution for DeFi liquidity provision. By managing liquidity in discrete intervals, adjusting prices minimally during swaps, and distributing fees fairly between participants, the pool maintains a stable and efficient marketplace for token swaps. This scalable approach not only supports current trading activities but also scales effectively as more liquidity providers and traders participate in the ecosystem.

## Further Details
For further details and a deeper understanding of the underlying mechanisms of the Precision Pool, it is recommended to also consult the Uniswap V3 whitepaper. The Precision Pool's design and operational principles are heavily inspired by the innovations introduced in Uniswap V3, particularly in terms of concentrated liquidity and dynamic fee structures. The whitepaper can be accessed at [Uniswap V3 Whitepaper](https://uniswap.org/whitepaper-v3.pdf), which provides comprehensive insights into the mathematical models and algorithms that are foundational to the functionalities described in this document.
