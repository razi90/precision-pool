use crate::{constants::*, pool_math::*, utils::*};
use common::math::*;
use common::metadata::{assert_component_packages_are_approved, assert_components_are_approved};
use common::pools::{token_symbol, SwapType};
use common::time::*;
use common::utils::assert_within_bounds;
use oracle::{AccumulatedObservation, ObservationInterval, Oracle};
use precision_pool_hooks::*;
use scrypto::prelude::*;
use scrypto_avltree::*;
use std::ops::Bound;

#[blueprint]
#[events(
    InstantiateEvent,
    AddLiquidityEvent,
    RemoveLiquidityEvent,
    SwapEvent,
    ClaimFeesEvent,
    FlashLoanEvent
)]
mod precision_pool {
    enable_method_auth! {
        roles {
            blueprint => updatable_by: [];
        },
        methods {
            swap                        => PUBLIC;
            add_liquidity               => PUBLIC;
            add_liquidity_shape         => PUBLIC;
            remove_liquidity            => PUBLIC;
            removable_liquidity         => PUBLIC;
            tick_spacing                => PUBLIC;
            x_address                   => PUBLIC;
            y_address                   => PUBLIC;
            x_divisibility              => PUBLIC;
            y_divisibility              => PUBLIC;
            total_liquidity             => PUBLIC;
            lp_address                  => PUBLIC;
            price_sqrt                  => PUBLIC;
            active_tick                 => PUBLIC;
            input_fee_rate              => PUBLIC;
            fee_protocol_share          => PUBLIC;
            flash_loan_fee_rate         => PUBLIC;
            flash_loan                  => PUBLIC;
            repay_loan                  => PUBLIC;
            flash_loan_address          => PUBLIC;
            hook                        => PUBLIC;
            claim_fees                  => PUBLIC;
            claimable_fees              => PUBLIC;
            seconds_in_position         => PUBLIC;
            total_fees                  => PUBLIC;
            registry                    => PUBLIC;
            sync_registry               => PUBLIC;
            next_sync_time              => PUBLIC;
            observations_limit          => PUBLIC;
            observation                 => PUBLIC;
            oldest_observation_at       => PUBLIC;
            observation_intervals       => PUBLIC;
            observations_stored         => PUBLIC;
            last_observation_index      => PUBLIC;
            execute_after_instantiate   => restrict_to: [blueprint];
        }
    }

    struct PrecisionPool {
        pool_address: ComponentAddress,

        x_liquidity: Vault,
        y_liquidity: Vault,

        x_fees: Vault,
        y_fees: Vault,

        tick_spacing: u32,
        max_liquidity_per_tick: PreciseDecimal,

        price_sqrt: PreciseDecimal,
        active_tick: Option<i32>,
        active_liquidity: PreciseDecimal,

        lp_manager: ResourceManager,
        lp_counter: u64,

        ticks: AvlTree<i32, Tick>,

        registry: Global<AnyComponent>,
        next_sync_time: u64,

        input_fee_rate: Decimal,
        fee_protocol_share: Decimal,

        x_lp_fee: PreciseDecimal,
        y_lp_fee: PreciseDecimal,
        x_protocol_fee: Vault,
        y_protocol_fee: Vault,
        instantiated_at: u64,

        flash_manager: ResourceManager,
        flash_loan_fee_rate: Decimal,

        hooks: HashMap<(PackageAddress, String), Global<AnyComponent>>,
        hook_calls: HookCalls,
        hook_badges: HashMap<ComponentAddress, Vault>,

        oracle: Oracle,
    }

    impl PrecisionPool {
        /// Instantiates a new `PrecisionPool` with specified parameters.
        ///
        /// This method sets up a new liquidity pool with concentrated liquidity features. It ensures that the provided
        /// token addresses are valid and different, and that the initial price is within the acceptable bounds. It also
        /// sets up various fees and hooks for additional functionalities.
        ///
        /// ## Arguments
        /// - `x_address`: ResourceAddress for token X used in this pool.
        /// - `y_address`: ResourceAddress for token Y used in this pool.
        /// - `price_sqrt`: Initial square root of the price for this pool.
        /// - `tick_spacing`: The spacing between ticks in the pool, affecting the granularity of price changes.
        /// - `input_fee_rate`: Fee rate applied to swap inputs.
        /// - `flash_loan_fee_rate`: Fee rate applied to flash loans.
        /// - `registry_address`: Address of the registry for protocol fee configuration and collection.
        /// - `hook_badges`: Vector of tuples containing hook components and associated badges for access control.
        /// - `dapp_definition`: Address of the dApp definition component.
        ///
        /// ## Returns
        /// A tuple containing:
        /// - A global reference to the instantiated `PrecisionPool`.
        /// - The resource address of the LP tokens linked to this pool.
        ///
        /// ## Panics
        /// - If `x_address` and `y_address` are the same.
        /// - If `x_address` is not less than `y_address`.
        /// - If `price_sqrt` is not positive or outside the allowed tick bounds.
        /// - If `input_fee_rate` or `flash_loan_fee_rate` are not within valid ranges.
        /// - If either `x_address` or `y_address` do not point to fungible tokens.
        pub fn instantiate(
            x_address: ResourceAddress,
            y_address: ResourceAddress,
            price_sqrt: PreciseDecimal,
            tick_spacing: u32,
            input_fee_rate: Decimal,
            flash_loan_fee_rate: Decimal,
            registry_address: ComponentAddress,
            hook_badges: Vec<(ComponentAddress, Bucket)>,
            dapp_definition: ComponentAddress,
        ) -> (Global<PrecisionPool>, ResourceAddress) {
            // Ensure token addresses are valid and different to prevent erroneous pool behavior.
            assert_ne!(
                x_address, y_address,
                "[Instantiate]: The token addresses must be different."
            );
            // Ensure token addresses are sorted to maintain consistent ordering for pool operations.
            assert!(
                x_address < y_address,
                "[Instantiate]: Addresses are not sorted: y_address < x_address"
            );

            // Validate tick spacing within predefined bounds.
            assert_within_bounds(tick_spacing, 1, MAX_TICK as u32, "tick_spacing");

            // Ensure the initial price is positive and within the acceptable range.
            assert!(
                price_sqrt > pdec!(0),
                "[Instantiate]: The price_sqrt must be positive."
            );
            assert_within_bounds(
                price_sqrt,
                tick_to_price_sqrt(MIN_TICK),
                tick_to_price_sqrt(MAX_TICK),
                "price_sqrt",
            );

            // Validate fee rates for input and flash loans.
            assert_input_fee_rate_is_valid(input_fee_rate);
            assert_flash_loan_fee_rate_is_valid(flash_loan_fee_rate);

            // Ensure both token addresses point to fungible tokens.
            assert!(
                ResourceManager::from_address(x_address)
                    .resource_type()
                    .is_fungible(),
                "[Instantiate]: Address A should be a fungible token."
            );
            assert!(
                ResourceManager::from_address(y_address)
                    .resource_type()
                    .is_fungible(),
                "[Instantiate]: Address B should be a fungible token."
            );

            assert_components_are_approved("registry_components", vec![registry_address]);
            assert_component_packages_are_approved(
                "hook_packages",
                hook_badges.iter().map(|(address, _)| *address).collect(),
            );

            // Generate and execute hooks for additional functionalities before instantiation.
            let (hook_calls, mut hook_badges_bucket, hooks) = generate_calls_hooks(hook_badges);
            execute_hooks_before_instantiate(
                &hook_calls.before_instantiate,
                &hook_badges_bucket,
                (BeforeInstantiateState {
                    x_address,
                    y_address,
                    price_sqrt: Some(price_sqrt),
                    input_fee_rate,
                    flash_loan_fee_rate,
                },),
            );

            // Move hook badges from buckets to vaults to store in the component state.
            let hook_badges_vault: HashMap<ComponentAddress, Vault> = hook_badges_bucket
                .drain()
                .map(|(component_address, bucket)| (component_address, Vault::with_bucket(bucket)))
                .collect();

            // Initialize vaults for liquidity of token X and Y.
            let x_liquidity: Vault = Vault::new(x_address);
            let y_liquidity: Vault = Vault::new(y_address);

            // Reserve an address for the new pool and set up LP token management.
            let (address_reservation, pool_address) =
                Runtime::allocate_component_address(PrecisionPool::blueprint_id());
            let (pool_name, lp_name, lp_description) =
                Self::names_and_lp_description(x_address, y_address);
            let lp_manager =
                Self::set_lp_manager(pool_address, lp_name, lp_description, dapp_definition);

            // Set up a resource manager for flash loans.
            let flash_manager =
                ResourceBuilder::new_ruid_non_fungible::<FlashLoan>(OwnerRole::None)
                    .mint_roles(mint_roles!(
                        minter => rule!(require(global_caller(pool_address)));
                        minter_updater => rule!(deny_all);
                    ))
                    .burn_roles(burn_roles!(
                        burner => rule!(require(global_caller(pool_address)));
                        burner_updater => rule!(deny_all);
                    ))
                    .deposit_roles(deposit_roles!(
                        depositor => rule!(deny_all);
                        depositor_updater => rule!(deny_all);
                    ))
                    .create_with_no_initial_supply();

            // Collect hook addresses for metadata.
            let hooks_vec: Vec<ComponentAddress> = hook_badges_vault.keys().cloned().collect();

            // Instantiate the pool and set its initial state and metadata.
            let pool = (Self {
                x_liquidity,
                y_liquidity,
                x_fees: Vault::new(x_address),
                y_fees: Vault::new(y_address),
                price_sqrt,
                tick_spacing,
                max_liquidity_per_tick: max_liquidity_per_tick(tick_spacing),
                lp_manager,
                lp_counter: 0,
                ticks: AvlTree::new(),
                active_tick: None,
                active_liquidity: PreciseDecimal::ZERO,
                registry: registry_address.into(),
                next_sync_time: 0,
                pool_address,
                input_fee_rate,
                flash_loan_fee_rate,
                fee_protocol_share: dec!(0),
                x_lp_fee: pdec!(0),
                y_lp_fee: pdec!(0),
                x_protocol_fee: Vault::new(x_address),
                y_protocol_fee: Vault::new(y_address),
                instantiated_at: Clock::time_in_seconds(),
                flash_manager,
                hook_calls,
                hook_badges: hook_badges_vault,
                hooks,
                oracle: Oracle::new(u16::MAX),
            })
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .roles(roles!(
                blueprint => rule!(require(global_caller(PrecisionPool::blueprint_id())));
            ))
            .with_address(address_reservation)
            .metadata(metadata! {
                init {
                    "pool_address" => pool_address, locked;
                    "name" => pool_name, locked;
                    "lp_address" => lp_manager.address(), locked;
                    "x_address" => x_address, locked;
                    "y_address" => y_address, locked;
                    "flash_loan_address" => flash_manager.address(), locked;
                    "tick_spacing" => tick_spacing, locked;
                    "input_fee_rate" => input_fee_rate, locked;
                    "flash_loan_fee_rate" => flash_loan_fee_rate, locked;
                    "registry" => registry_address, locked;
                    "hooks" => hooks_vec.clone(), locked;
                    "dapp_definition" => dapp_definition, locked;
                }
            })
            .globalize();

            // Execute post-instantiation hooks and emit an event for successful instantiation.
            pool.execute_after_instantiate(AfterInstantiateState {
                pool_address,
                x_address,
                y_address,
                price_sqrt: Some(price_sqrt),
                input_fee_rate,
                flash_loan_fee_rate,
            });

            Runtime::emit_event(InstantiateEvent {
                pool_address,
                lp_address: lp_manager.address(),
                x_address,
                y_address,
                flash_loan_address: flash_manager.address(),
                price_sqrt,
                tick_spacing,
                input_fee_rate,
                flash_loan_fee_rate,
                registry_address,
                hooks: hooks_vec,
                dapp_definition,
            });

            (pool, lp_manager.address())
        }

        /// Instantiates a new Precision Pool with initial liquidity.
        ///
        /// This method leverages the `instantiate` method to create a new `PrecisionPool` and then adds initial
        /// liquidity within specified price bounds. It allows for setting up a ready-to-use Precision Pool
        /// including initial liquidity in one transaction.
        ///
        /// ## Arguments
        /// - `x_bucket`: Bucket containing token X for initial liquidity.
        /// - `y_bucket`: Bucket containing token Y for initial liquidity.
        /// - `price_sqrt`: The initial square root of the price, determining the initial price ratio of X to Y.
        /// - `tick_spacing`: The minimum price movement this pool supports.
        /// - `input_fee_rate`: The fee rate charged on swaps (e.g., 0.03 for 3%).
        /// - `flash_loan_fee_rate`: The fee rate charged on flash loans.
        /// - `registry_address`: The address of the registry for managing protocol configurations.
        /// - `hook_badges`: A vector of tuples pairing component addresses with badges, controlling access to callable hooks.
        /// - `dapp_definition`: The address of the decentralized application (dApp) associated with this pool.
        /// - `left_bound`: The lower price bound for adding liquidity.
        /// - `right_bound`: The upper price bound for adding liquidity.
        ///
        /// ## Returns
        /// - A tuple containing:
        ///   - A global reference to the instantiated `PrecisionPool`.
        ///   - A bucket containing the LP tokens representing the initial liquidity position.
        ///   - Two buckets containing any remaining unallocated tokens X and Y.
        ///
        /// ## Panics
        /// - If the `left_bound` is greater than or equal to `right_bound`.
        /// - If the token addresses for X and Y are the same or not in ascending order.
        /// - If the `price_sqrt` is not positive or outside the allowed tick bounds.
        /// - If `input_fee_rate` or `flash_loan_fee_rate` are not within valid ranges.
        pub fn instantiate_with_liquidity(
            x_bucket: Bucket,
            y_bucket: Bucket,
            price_sqrt: PreciseDecimal,
            tick_spacing: u32,
            input_fee_rate: Decimal,
            flash_loan_fee_rate: Decimal,
            registry_address: ComponentAddress,
            hook_badges: Vec<(ComponentAddress, Bucket)>,
            dapp_definition: ComponentAddress,
            left_bound: i32,
            right_bound: i32,
        ) -> (Global<PrecisionPool>, Bucket, Bucket, Bucket) {
            let (pool, _) = Self::instantiate(
                x_bucket.resource_address(),
                y_bucket.resource_address(),
                price_sqrt,
                tick_spacing,
                input_fee_rate,
                flash_loan_fee_rate,
                registry_address,
                hook_badges,
                dapp_definition,
            );
            let (liquidity_position, x_bucket, y_bucket) =
                pool.add_liquidity(left_bound, right_bound, x_bucket, y_bucket);
            (pool, liquidity_position, x_bucket, y_bucket)
        }

        /// Executes post-instantiation hooks to extend pool functionality.
        ///
        /// # Arguments
        /// * `after_instantiate_state`: The state information specific to this pool that will be passed to the hooks for processing.
        pub fn execute_after_instantiate(&self, after_instantiate_state: AfterInstantiateState) {
            self.execute_hooks(HookCall::AfterInstantiate, (after_instantiate_state,));
        }

        /// Internal function to add liquidity within specified price bounds.
        ///
        /// This function performs the core logic for adding liquidity, including aligning price bounds to valid ticks,
        /// validating token addresses, and calculating the maximum amounts of tokens that can be added. It also handles
        /// the minting of LP tokens and updates the pool's state.
        ///
        /// # Arguments
        /// * `left_bound` - The lower price bound for adding liquidity.
        /// * `right_bound` - The upper price bound for adding liquidity.
        /// * `x_bucket` - A bucket containing the X tokens to be added as liquidity.
        /// * `y_bucket` - A bucket containing the Y tokens to be added as liquidity.
        /// * `shape_id` - An optional identifier for a specific liquidity shape, used for more complex liquidity strategies.
        ///
        /// # Returns
        /// A tuple containing:
        /// * A bucket with any remaining X tokens not added as liquidity.
        /// * A bucket with any remaining Y tokens not added as liquidity.
        /// * A bucket containing the LP tokens representing the added liquidity.
        ///
        /// # Panics
        /// - If `left_bound` or `right_bound` are outside the allowed tick range.
        /// - If `left_bound` is not less than `right_bound`.
        /// - If the X or Y token addresses do not match the expected addresses for this pool.
        fn add_liquidity_internal(
            &mut self,
            left_bound: i32,
            right_bound: i32,
            mut x_bucket: Bucket,
            mut y_bucket: Bucket,
            shape_id: Option<NonFungibleLocalId>,
        ) -> (Bucket, Bucket, Bucket) {
            // Ensure the bounds are within the allowed tick range.
            assert!(left_bound >= MIN_TICK, "Left bound lower than allowed.");
            assert!(right_bound <= MAX_TICK, "Right bound higher than allowed.");

            // Align the bounds to the nearest valid ticks based on the pool's tick spacing.
            let left_bound = align_tick(left_bound, self.tick_spacing);
            let right_bound = align_tick(right_bound, self.tick_spacing);

            // Ensure the left bound is strictly less than the right bound after alignment.
            assert!(
                left_bound < right_bound,
                "Please ensure that the provided left bound is strictly smaller than the right bound, and that they are not mapped to the same tick."
            );

            // Validate that the token addresses in the buckets match the expected addresses for this pool.
            assert_eq!(
                x_bucket.resource_address(),
                self.x_address(),
                "[Add liquidity]: The X token address is not correct."
            );
            assert_eq!(
                y_bucket.resource_address(),
                self.y_address(),
                "[Add liquidity]: The Y token address is not correct."
            );

            // Capture the initial amounts of X and Y tokens provided for liquidity.
            let (x_provided, y_provided) = (x_bucket.amount(), y_bucket.amount());

            // Prepare the state before adding liquidity, including the current pool state and the position details.
            let state_before = BeforeAddLiquidityState {
                pool_address: self.pool_address,
                x_provided,
                y_provided,
                active_liquidity: self.active_liquidity,
                price_sqrt: self.price_sqrt,
                position: LiquidityPositionType {
                    left_bound,
                    right_bound,
                    position_id: None,
                    shape_id: shape_id.clone(),
                },
            };

            // Execute hooks before adding liquidity, allowing for custom logic or validations.
            (_, x_bucket, y_bucket) = self.execute_hooks(
                HookCall::BeforeAddLiquidity,
                (state_before, x_bucket, y_bucket),
            );

            // Verify that the hooks have not improperly modified the amounts of X and Y tokens.
            assert_hooks_bucket_output(x_provided, x_bucket.amount(), "BeforeAddLiquidity");
            assert_hooks_bucket_output(y_provided, y_bucket.amount(), "BeforeAddLiquidity");

            // Calculate the square root prices for the left and right bounds.
            let price_left_sqrt = tick_to_price_sqrt(left_bound);
            let price_right_sqrt = tick_to_price_sqrt(right_bound);

            // Determine the maximum amounts of X and Y tokens that can be added as liquidity at the current price.
            let (liquidity, x_amount, y_amount) = addable_amounts(
                x_bucket.amount(),
                self.x_divisibility(),
                y_bucket.amount(),
                self.y_divisibility(),
                self.price_sqrt,
                price_left_sqrt,
                price_right_sqrt,
            );

            // Ensure that the calculated liquidity is not zero.
            assert_ne!(
                liquidity,
                pdec!(0),
                "[Add liquidity]: Allowed liquidity is zero."
            );

            // Deposit the calculated amounts of X and Y tokens into the pool's liquidity vaults.
            self.x_liquidity.put(x_bucket.take(x_amount));
            self.y_liquidity.put(y_bucket.take(y_amount));

            // Update the pool's active liquidity and active tick based on the added liquidity.
            self.update_active_liquidity(liquidity, price_left_sqrt, price_right_sqrt);
            self.update_active_tick(left_bound, right_bound, price_left_sqrt, price_right_sqrt);

            // Update or insert tick data for the left and right bounds.
            let left_tick = self.update_or_insert_tick(left_bound, liquidity, liquidity);
            let right_tick = self.update_or_insert_tick(right_bound, -liquidity, liquidity);

            // Calculate fee checkpoints for X and Y tokens based on the updated tick data.
            // The checkpoints can be negative since only the growth between two checkpoints is relevant.
            let x_fee_checkpoint = value_in_range(
                self.x_lp_fee,
                left_tick.x_fee_outside,
                right_tick.x_fee_outside,
                self.active_tick,
                left_bound,
                right_bound,
            );
            let y_fee_checkpoint = value_in_range(
                self.y_lp_fee,
                left_tick.y_fee_outside,
                right_tick.y_fee_outside,
                self.active_tick,
                left_bound,
                right_bound,
            );

            // Calculate the seconds spent inside the current liquidity position.
            // The checkpoint can be negative and works analog to fee checkpoints.
            // The difference between two checkpoints gives the "growth" in seconds.
            let seconds_inside_checkpoint = value_in_range(
                self.seconds_global() as i64,
                left_tick.seconds_outside as i64,
                right_tick.seconds_outside as i64,
                self.active_tick,
                left_bound,
                right_bound,
            );

            // Mint LP tokens representing the added liquidity and capture the new position details.
            let (position_id, position, mut position_bucket) = self.mint_lp_token(
                liquidity,
                left_bound,
                right_bound,
                shape_id.clone(),
                x_fee_checkpoint,
                y_fee_checkpoint,
                seconds_inside_checkpoint,
            );

            let state_after = AfterAddLiquidityState {
                pool_address: self.pool_address,
                x_added: x_amount,
                y_added: y_amount,
                added_liquidity: liquidity,
                active_liquidity: self.active_liquidity,
                price_sqrt: self.price_sqrt,
                position: LiquidityPositionType {
                    left_bound,
                    right_bound,
                    position_id: Some(position_id.clone()),
                    shape_id,
                },
            };

            // Execute hooks after adding liquidity, allowing for custom logic or validations.
            (_, position_bucket) =
                self.execute_hooks(HookCall::AfterAddLiquidity, (state_after, position_bucket));

            Runtime::emit_event(AddLiquidityEvent {
                position_id,
                position,
                left_tick,
                right_tick,
                x_amount,
                y_amount,
                x_gross_amount: x_provided - x_bucket.amount(), // can be negative if the before add liquidity hook puts x into the bucket
                y_gross_amount: y_provided - y_bucket.amount(), // can be negative if the before add liquidity hook puts y into the bucket
                active_liquidity: self.active_liquidity,
                active_tick: self.active_tick,
            });

            // Return the buckets with remaining X and Y tokens, and the bucket with the minted LP tokens.
            (position_bucket, x_bucket, y_bucket)
        }

        /// Adds liquidity to the pool within specified price bounds.
        ///
        /// This method is responsible for adding liquidity to the pool between the specified `left_bound` and `right_bound`.
        /// It aligns these bounds to the nearest valid ticks, checks for token address correctness, and calculates the maximum
        /// amounts of tokens that can be added as liquidity at the current price. It also handles the minting of LP tokens
        /// and updates the pool's state accordingly.
        ///
        /// # Arguments
        /// * `left_bound` - The lower price bound for adding liquidity.
        /// * `right_bound` - The upper price bound for adding liquidity.
        /// * `x_bucket` - A bucket containing the X tokens to be added as liquidity.
        /// * `y_bucket` - A bucket containing the Y tokens to be added as liquidity.
        /// * `shape_id` - An optional identifier for a specific liquidity shape, used for more complex liquidity strategies.
        ///
        /// # Returns
        /// A tuple containing:
        /// * A bucket with any remaining X tokens not added as liquidity.
        /// * A bucket with any remaining Y tokens not added as liquidity.
        /// * A bucket containing the LP tokens representing the added liquidity.
        ///
        /// # Panics
        /// - If `left_bound` or `right_bound` are outside the allowed tick range.
        /// - If `left_bound` is not less than `right_bound`.
        /// - If the X or Y token addresses do not match the expected addresses for this pool.
        pub fn add_liquidity(
            &mut self,
            left_bound: i32,
            right_bound: i32,
            x_bucket: Bucket,
            y_bucket: Bucket,
        ) -> (Bucket, Bucket, Bucket) {
            self.add_liquidity_internal(left_bound, right_bound, x_bucket, y_bucket, None)
        }

        /// Adds multiple liquidity positions to the pool simultaneously.
        ///
        /// This method allows for batch processing of multiple liquidity additions, which can be more gas-efficient
        /// and ensures atomicity of transactions. It is particularly useful for strategies involving multiple liquidity
        /// positions across different price ranges.
        ///
        /// # Arguments
        /// - `positions`: A vector of tuples, each representing a liquidity position with:
        ///   - `left_bound`: The lower price bound for adding liquidity.
        ///   - `right_bound`: The upper price bound for adding liquidity.
        ///   - `x_bucket`: A bucket containing the X tokens to be added as liquidity.
        ///   - `y_bucket`: A bucket containing the Y tokens to be added as liquidity.
        /// - `shape_proof`: An optional non-fungible proof that allows for adding new positions to an existing shape.
        ///
        /// # Returns
        /// A tuple containing:
        /// - `Bucket`: A bucket with any remaining X tokens not added as liquidity across all positions.
        /// - `Bucket`: A bucket with any remaining Y tokens not added as liquidity across all positions.
        /// - `Bucket`: A bucket containing the LP tokens representing the added liquidity for all positions.
        pub fn add_liquidity_shape(
            &mut self,
            positions: Vec<(i32, i32, Bucket, Bucket)>,
            shape_proof: Option<NonFungibleProof>,
        ) -> (Bucket, Bucket, Bucket) {
            // Initialize output buckets for X and Y tokens using the pool's respective token addresses.
            let mut x_output_shape = Bucket::new(self.x_address());
            let mut y_output_shape = Bucket::new(self.y_address());
            // Initialize an LP token bucket using the LP manager's address.
            let mut lp_nfts = Bucket::new(self.lp_manager.address());
            // Determine the `shape_id` based on the provided `shape_proof`, generating a new one if not provided.
            let shape_id = match shape_proof {
                Some(proof) => {
                    proof
                        .check(self.lp_manager.address())
                        .non_fungible::<LiquidityPosition>()
                        .data()
                        .shape_id
                }
                None => Some(NonFungibleLocalId::ruid(Runtime::generate_ruid())),
            };
            // Process each position, collecting resulting LP tokens and any unadded X and Y tokens.
            for (left_bound, right_bound, x_bucket, y_bucket) in positions {
                let (lp_nft, x_output, y_output) = self.add_liquidity_internal(
                    left_bound,
                    right_bound,
                    x_bucket,
                    y_bucket,
                    shape_id.clone(),
                );
                lp_nfts.put(lp_nft);
                x_output_shape.put(x_output);
                y_output_shape.put(y_output);
            }
            // Aggregate all LP tokens and unadded tokens into their respective buckets.
            (lp_nfts, x_output_shape, y_output_shape)
        }

        /// Mints a new liquidity position non-fungible token (LP NFT).
        ///
        /// This function is responsible for creating a new LP NFT that represents a liquidity position within the pool.
        /// It increments the LP counter to ensure a unique identifier for each new position, encapsulates the liquidity details
        /// in a `LiquidityPosition` struct, and mints the NFT using the pool's LP manager.
        ///
        /// ## Arguments
        /// - `liquidity`: The amount of liquidity being added.
        /// - `left_bound`: The lower price bound of the liquidity position.
        /// - `right_bound`: The upper price bound of the liquidity position.
        /// - `shape_id`: An optional identifier for a specific liquidity shape, used for more complex liquidity strategies.
        /// - `x_fee_checkpoint`: The checkpoint for fees collected in token X up to this point.
        /// - `y_fee_checkpoint`: The checkpoint for fees collected in token Y up to this point.
        /// - `seconds_inside_checkpoint`: The checkpoint for total time in seconds this position has been active within the specified price range.
        ///
        /// ## Returns
        /// - `NonFungibleLocalId`: The unique identifier of the newly minted LP NFT.
        /// - `LiquidityPosition`: The liquidity position data encapsulated in the minted NFT.
        /// - `Bucket`: A bucket containing the newly minted LP NFT.
        fn mint_lp_token(
            &mut self,
            liquidity: PreciseDecimal,
            left_bound: i32,
            right_bound: i32,
            shape_id: Option<NonFungibleLocalId>,
            x_fee_checkpoint: PreciseDecimal,
            y_fee_checkpoint: PreciseDecimal,
            seconds_inside_checkpoint: i64,
        ) -> (NonFungibleLocalId, LiquidityPosition, Bucket) {
            // Increment the LP counter to generate a unique position ID.
            self.lp_counter += 1;
            let position_id =
                NonFungibleLocalId::Integer(IntegerNonFungibleLocalId::new(self.lp_counter));

            let position = LiquidityPosition {
                liquidity,
                left_bound,
                right_bound,
                shape_id,
                added_at: Clock::time_in_seconds(),
                x_fee_checkpoint,
                y_fee_checkpoint,
                x_total_fee_checkpoint: x_fee_checkpoint,
                y_total_fee_checkpoint: y_fee_checkpoint,
                seconds_inside_checkpoint,
            };

            // Mint the LP NFT using the LP manager and the newly created position.
            let position_bucket = self
                .lp_manager
                .mint_non_fungible(&position_id, position.clone());

            (position_id, position, position_bucket)
        }

        /// Updates the active liquidity in the pool based on the current price.
        ///
        /// Adjusts the pool's active liquidity by adding `liquidity` when the current price (`self.price_sqrt`)
        /// is between `price_left_sqrt` (inclusive) and `price_right_sqrt` (exclusive).
        ///
        /// ## Arguments
        /// - `liquidity`: The amount of liquidity to potentially add to active liquidity.
        /// - `price_left_sqrt`: The square root of the lower price bound.
        /// - `price_right_sqrt`: The square root of the upper price bound.
        fn update_active_liquidity(
            &mut self,
            liquidity: PreciseDecimal,
            price_left_sqrt: PreciseDecimal,
            price_right_sqrt: PreciseDecimal,
        ) {
            // Check if the current price is within the specified bounds.
            // If it is not, exit the function without modifying the active liquidity.
            if self.price_sqrt < price_left_sqrt || price_right_sqrt <= self.price_sqrt {
                return;
            }
            // If the current price is within the bounds, add the specified liquidity
            // to the pool's active liquidity.
            self.active_liquidity += liquidity;
        }

        /// Updates the active tick index based on the current price and given bounds.
        ///
        /// Adjusts the `active_tick` of the pool to the nearest tick that is less than or equal to the current pool price.
        ///
        /// ## Arguments
        /// - `left_bound`: The lower bound tick index for the update.
        /// - `right_bound`: The upper bound tick index for the update.
        /// - `price_left_sqrt`: The square root of the price corresponding to the `left_bound`.
        /// - `price_right_sqrt`: The square root of the price corresponding to the `right_bound`.
        fn update_active_tick(
            &mut self,
            left_bound: i32,
            right_bound: i32,
            price_left_sqrt: PreciseDecimal,
            price_right_sqrt: PreciseDecimal,
        ) {
            // If active tick is None we set it to the integer minimum since every valid left or right bound
            // is a better fit than the smallest possible active tick
            let active_tick = self.active_tick.unwrap_or(i32::MIN);

            // Update the active tick to the right bound if the right bound is closest left of the price.
            if active_tick < right_bound && price_right_sqrt <= self.price_sqrt {
                self.active_tick = Some(right_bound);
            }
            // Update the active tick to the left bound if the left bound is closest left of the price.
            else if active_tick < left_bound && price_left_sqrt <= self.price_sqrt {
                self.active_tick = Some(left_bound);
            }
        }

        /// Refits the `active_tick` to the highest tick less than or equal to the current `active_tick`.
        ///
        /// This method adjusts the `active_tick` to the last valid tick that is not greater than the current `active_tick`.
        /// It is used to ensure that the `active_tick` remains aligned with the existing ticks after any updates or changes
        /// that might affect the tick indices. This is especially needed after deleting a tick.
        fn refit_active_tick(&mut self) {
            // Set active_tick to the highest tick not greater than the current active_tick
            self.active_tick = self
                .ticks
                .range_back((
                    Bound::Unbounded,
                    Bound::Included(self.active_tick.unwrap_or(i32::MIN)),
                ))
                .next()
                .map(|(key, _, _)| key);
        }

        /// Calculates the removable amounts of token X and token Y from the pool without removing liquidity.
        ///
        /// This method is called to determine the amounts of token X and token Y that can be withdrawn from the pool
        /// based on the current pool state and the specifics of the liquidity position represented by the NFTs in `lp_position_ids`.
        ///
        /// Note: If remove liquidity hooks are utilized, they are not accounted for in this calculation and may alter the final output.
        /// The minimum removable fraction indicates the minimum fraction of liquidity that can be withdrawn.
        ///
        /// # Arguments
        /// * `lp_position_ids`: A vector containing the IDs of liquidity position NFTs which represent the liquidity
        ///   positions to be evaluated.
        ///
        /// # Returns
        /// A tuple consisting of:
        /// * `IndexMap<ResourceAddress, Decimal>` - A map containing the resource addresses and the corresponding amount that can be removed from the pool.
        /// * The minimum removable fraction of the liquidity after hooks are executed.
        pub fn removable_liquidity(
            &self,
            lp_position_ids: Vec<NonFungibleLocalId>,
        ) -> (IndexMap<ResourceAddress, Decimal>, Decimal) {
            let mut x_total_output = dec!(0);
            let mut y_total_output = dec!(0);

            for position_id in lp_position_ids {
                let position: LiquidityPosition =
                    self.lp_manager.get_non_fungible_data(&position_id);
                let price_left_sqrt = tick_to_price_sqrt(position.left_bound);
                let price_right_sqrt = tick_to_price_sqrt(position.right_bound);

                let (x_fees, y_fees, _, _) = self.claimable_fees_internal(&position);
                let (x_amount, y_amount) = removable_amounts(
                    position.liquidity,
                    self.price_sqrt,
                    price_left_sqrt,
                    price_right_sqrt,
                    self.x_divisibility(),
                    self.y_divisibility(),
                );

                x_total_output += x_fees;
                y_total_output += y_fees;
                x_total_output += x_amount;
                y_total_output += y_amount;
            }

            let minimum_removable_fraction =
                match self.hook_calls.after_remove_liquidity.1.is_empty() {
                    true => dec!(1),
                    false => HOOKS_MIN_REMAINING_BUCKET_FRACTION,
                };

            (
                IndexMap::from([
                    (self.x_liquidity.resource_address(), x_total_output),
                    (self.y_liquidity.resource_address(), y_total_output),
                ]),
                minimum_removable_fraction,
            )
        }

        /// Removes liquidity from the pool and returns the corresponding amounts of token X and token Y.
        ///
        /// This method is called when a liquidity provider wants to withdraw their liquidity from the pool.
        /// It calculates the amounts of token A and token B to be returned to the provider based on the current
        /// pool state and the specifics of the liquidity position represented by the NFTs in `lp_positions`.
        ///
        /// # Arguments
        /// * `lp_positions`: A non-fungible bucket containing liquidity position NFTs which represent the liquidity
        ///   positions to be removed.
        ///
        /// # Returns
        /// A tuple consisting of:
        /// * A bucket with the token X from the pool
        /// * A bucket with the token Y from the pool
        pub fn remove_liquidity(&mut self, lp_positions: NonFungibleBucket) -> (Bucket, Bucket) {
            // Initialize output buckets for tokens X and Y for all positions.
            let mut x_total_output = Bucket::new(self.x_address());
            let mut y_total_output = Bucket::new(self.y_address());

            // Iterate over each NFT in `lp_positions`:
            for nft in lp_positions.non_fungibles::<LiquidityPosition>() {
                let position = nft.data();
                let price_left_sqrt = tick_to_price_sqrt(position.left_bound);
                let price_right_sqrt = tick_to_price_sqrt(position.right_bound);

                // Execute pre-removal hooks.
                let state_before = BeforeRemoveLiquidityState {
                    pool_address: self.pool_address,
                    provided_liquidity: position.liquidity,
                    active_liquidity: self.active_liquidity,
                    price_sqrt: self.price_sqrt,
                    position: LiquidityPositionType {
                        left_bound: position.left_bound,
                        right_bound: position.right_bound,
                        position_id: Some(nft.local_id().clone()),
                        shape_id: position.shape_id.clone(),
                    },
                };
                let _ = self.execute_hooks(HookCall::BeforeRemoveLiquidity, (state_before,));

                // Auto-claim fees before removing liquidity position
                let (x_fees, y_fees) = self.claim_fees_internal(&nft);
                // Calculate the token amounts to be removed based on the liquidity position data.
                let (x_amount, y_amount) = removable_amounts(
                    position.liquidity,
                    self.price_sqrt,
                    price_left_sqrt,
                    price_right_sqrt,
                    self.x_divisibility(),
                    self.y_divisibility(),
                );

                // Update the ticks and active liquidity of the pool.
                let left_tick = self.update_or_remove_tick(
                    position.left_bound,
                    -position.liquidity,
                    -position.liquidity,
                );
                let right_tick = self.update_or_remove_tick(
                    position.right_bound,
                    position.liquidity,
                    -position.liquidity,
                );
                self.update_active_liquidity(
                    -position.liquidity,
                    price_left_sqrt,
                    price_right_sqrt,
                );
                self.refit_active_tick();

                // Remove the specified liquidity amounts from the pool's liquidity vaults.
                let mut x_output = self.x_liquidity.take(x_amount);
                let mut y_output = self.y_liquidity.take(y_amount);

                // Execute post-removal hooks.
                let state_after = AfterRemoveLiquidityState {
                    pool_address: self.pool_address,
                    x_removed: x_amount,
                    y_removed: y_amount,
                    removed_liquidity: position.liquidity,
                    active_liquidity: self.active_liquidity,
                    price_sqrt: self.price_sqrt,
                    position: LiquidityPositionType {
                        left_bound: position.left_bound,
                        right_bound: position.right_bound,
                        position_id: Some(nft.local_id().clone()),
                        shape_id: position.shape_id.clone(),
                    },
                };
                (_, x_output, y_output) = self.execute_hooks(
                    HookCall::AfterRemoveLiquidity,
                    (state_after, x_output, y_output),
                );

                assert_hooks_bucket_output(x_amount, x_output.amount(), "AfterRemoveLiquidity");
                assert_hooks_bucket_output(y_amount, y_output.amount(), "AfterRemoveLiquidity");

                Runtime::emit_event(RemoveLiquidityEvent {
                    position_id: nft.local_id().clone(),
                    position,
                    left_tick,
                    right_tick,
                    x_amount,
                    y_amount,
                    x_return_amount: x_output.amount(),
                    y_return_amount: y_output.amount(),
                    active_liquidity: self.active_liquidity,
                    active_tick: self.active_tick,
                });

                x_total_output.put(x_fees);
                y_total_output.put(y_fees);
                x_total_output.put(x_output);
                y_total_output.put(y_output);
            }

            // Burn the NFTs representing the removed liquidity positions.
            self.lp_manager.burn(lp_positions);

            (x_total_output, y_total_output)
        }

        fn before_swap_state(&self, swap_type: SwapType) -> BeforeSwapState {
            BeforeSwapState {
                pool_address: self.pool_address,
                price_sqrt: self.price_sqrt,
                active_liquidity: self.active_liquidity,
                swap_type,
                input_fee_rate: self.input_fee_rate,
                fee_protocol_share: self.fee_protocol_share,
            }
        }

        /// Executes a swap operation within the pool, handling liquidity adjustments, fee calculations, and tick crossing logic.
        ///
        /// # Arguments
        /// * `input_bucket`: A bucket containing tokens to be swapped.
        ///
        /// # Returns
        /// A tuple containing:
        /// * A bucket with the resulting tokens after the swap.
        /// * A bucket with any remaining tokens not used in the swap.
        ///
        /// This method orchestrates the swap process, including pre-swap state setup, fee calculations,
        /// liquidity adjustments, and post-swap state finalization. It leverages various sub-processes
        /// such as tick processing and fee handling to ensure the swap adheres to the pool's rules and
        /// configurations.
        pub fn swap(&mut self, mut input_bucket: Bucket) -> (Bucket, Bucket) {
            // Synchronize the pool's state with the registry to collect protocol fees and update protocol fee share from time to time.
            self.sync_registry();
            let input_gross_amount = input_bucket.amount();
            let swap_type = self.swap_type(input_bucket.resource_address());
            let mut before_state = self.before_swap_state(swap_type);
            (before_state, input_bucket) =
                self.execute_hooks(HookCall::BeforeSwap, (before_state, input_bucket));

            // Adjust the input fee rate based on the pre-swap hook output and validate the input amount.
            self.set_input_fee_rate(before_state.input_fee_rate);
            assert_hooks_bucket_output(input_gross_amount, input_bucket.amount(), "BeforeSwap");

            let (global_input_fee_lp, global_output_fee_lp) = self.global_fees(swap_type);

            /*
            The following invariants are valid:
                input_share + input_fee_rate = 1
                fee_lp_share + fee_protocol_share = 1
             */
            let input_divisibility = self.input_divisibility(swap_type);
            let (input_amount_net, input_fee_lp, input_fee_protocol) = input_amount_net(
                input_bucket.amount(),
                self.input_fee_rate,
                self.fee_protocol_share,
                input_divisibility,
            );

            // Initialize the swap state with the necessary parameters for processing the swap.
            let mut state = SwapState {
                pool_address: self.pool_address,
                input_address: input_bucket.resource_address(),
                output_address: self.output_address(swap_type),
                swap_type,
                output: dec!(0),
                output_divisibility: self.output_divisibility(swap_type),
                input: dec!(0),
                input_divisibility,
                remainder: input_amount_net,
                remainder_fee_lp: input_fee_lp,
                liquidity: self.active_liquidity,
                active_tick: self.active_tick,
                price_sqrt: self.price_sqrt,
                price_sqrt_sell_cache: None,
                input_fee_rate: self.input_fee_rate,
                fee_protocol_share: self.fee_protocol_share,
                fee_lp_share: dec!(1) - self.fee_protocol_share,
                input_share: dec!(1) - self.input_fee_rate,
                fee_lp_input: dec!(0),
                fee_protocol_input: dec!(0),
                fee_protocol_max: input_fee_protocol,
                global_input_fee_lp,
                global_output_fee_lp,
                global_seconds: self.seconds_global(),
                crossed_ticks: vec![],
            };

            // Ensure that the remainder is not empty before proceeding with tick processing.
            assert!(!state.remainder_is_empty());

            /*
            In case of SellX, we should only cross the tick if there is more liquidity somewhere to the left
            and we have a remainder left. Otherwise we are not crossing the tick.

            After each step or loop iteration all variables are in a safely rounded state (ceil/floor) regarding arithmetic precision errors.
            Therefore, we can treat each of them as fixed valid "constants" for the next step. This is especially relevant for the price_sqrt.
            Since in the next iteration all calculations use the same price_sqrt which is in a safe state no further precision corrections are necessary.
            */

            // Process ticks based on the swap type, adjusting the state accordingly.
            match swap_type {
                SwapType::BuyX => {
                    self.get_next_ticks(SwapType::BuyX).for_each(
                        |(&tick_index, tick, _next_tick_index)| {
                            buy_step(&mut state, tick, tick_index)
                        },
                    );
                }
                SwapType::SellX => {
                    self.get_next_ticks(SwapType::SellX).for_each(
                        |(&_tick_index, tick, next_tick_index)| {
                            sell_step(&mut state, tick, next_tick_index)
                        },
                    );
                }
            }

            // Finalize the swap by taking protocol fees and updating the pool's state.
            state.take_protocol_fees();
            self.active_liquidity = state.liquidity;
            self.active_tick = state.active_tick;
            self.price_sqrt = state.price_sqrt;

            let (mut output_bucket, input_bucket) =
                self.swap_deposit_and_withdraw(state.clone(), input_bucket);

            let mut after_state = AfterSwapState::from(state.clone());
            (after_state, output_bucket) =
                self.execute_hooks(HookCall::AfterSwap, (after_state, output_bucket));

            // Adjust the input fee rate based on the post-swap hook output and validate the output amount.
            self.set_input_fee_rate(after_state.input_fee_rate);
            assert_hooks_bucket_output(state.output, output_bucket.amount(), "AfterSwap");

            // Update the oracle with the new price square root.
            self.oracle.observe(state.price_sqrt);

            // Emit a swap event to log the swap details.
            Runtime::emit_event(SwapEvent {
                input_address: input_bucket.resource_address(),
                input_amount: state.input,
                input_gross_amount,
                output_address: output_bucket.resource_address(),
                output_amount: state.output,
                output_return_amount: output_bucket.amount(),
                input_fee_lp: state.fee_lp_input,
                input_fee_protocol: state.fee_protocol_input,
                price_sqrt: self.price_sqrt,
                active_liquidity: self.active_liquidity,
                active_tick: self.active_tick,
                global_x_fee_lp: self.x_lp_fee,
                global_y_fee_lp: self.y_lp_fee,
                crossed_ticks: state.crossed_ticks,
            });

            (output_bucket, input_bucket)
        }

        /// Retrieve the tick spacing of this pool
        ///
        /// # Returns
        /// * Tick spacing of this pool
        pub fn tick_spacing(&self) -> u32 {
            self.tick_spacing
        }

        /// Retrieve the address of the first token in this pool
        ///
        /// # Returns
        /// * Resource address of the first token in this pool
        pub fn x_address(&self) -> ResourceAddress {
            self.x_liquidity.resource_address()
        }

        /// Retrieve the address of the second token in this pool
        ///
        /// # Returns
        /// * Resource address of the second token in this pool
        pub fn y_address(&self) -> ResourceAddress {
            self.y_liquidity.resource_address()
        }

        /// Retrieves the global registry component associated with this pool.
        /// This registry is crucial as it configures and collects protocol fees,
        /// which are essential for the decentralized management and operational sustainability of the pool.
        ///
        /// # Returns
        /// * `Global<AnyComponent>` - A global reference to the registry component used by this pool.
        pub fn registry(&self) -> Global<AnyComponent> {
            self.registry
        }

        /// Synchronizes the pool's state with the registry to potentially update the protocol fees.
        ///
        /// This method is crucial for maintaining the pool's alignment with the broader protocol's fee structure,
        /// which may change over time due to governance actions or other external factors. It ensures that the pool
        /// operates with the most current fee settings, which is essential for correct fee distribution and protocol sustainability.
        ///
        /// If the current time is less than `next_sync_time`, the function exits early to throttle the frequency of updates,
        /// which helps in reducing unnecessary computations and state changes.
        pub fn sync_registry(&mut self) {
            // Check if the current time exceeds `next_sync_time` to prevent too frequent updates.
            if Clock::time_in_seconds() < self.next_sync_time {
                return;
            }

            // Calls the `sync` method on the registry component, passing the current pool address and the total protocol fees collected since the last sync.
            let (fee_protocol_share, next_sync_time) =
                self.registry
                    .call::<(ComponentAddress, Bucket, Bucket), (Decimal, u64)>(
                        "sync",
                        &(
                            self.pool_address,
                            self.x_protocol_fee.take_all(),
                            self.y_protocol_fee.take_all(),
                        ),
                    );

            // Updates the pool's state with the new protocol fee share and the next allowed sync time.
            self.set_fee_protocol_share(fee_protocol_share);
            self.next_sync_time = next_sync_time;
        }

        /// Returns the next scheduled synchronization time with the registry.
        ///
        /// This method provides the timestamp (in seconds since the Unix epoch) when the pool is next set to synchronize its state with the registry.
        ///
        /// # Returns
        /// * `u64` - The Unix timestamp indicating when the next synchronization with the registry is scheduled.
        pub fn next_sync_time(&self) -> u64 {
            self.next_sync_time
        }

        /// Sets the input fee rate for the pool.
        ///
        /// Updates the pool's `input_fee_rate` after validating it, ensuring correct fee calculations for transactions.
        ///
        /// # Arguments
        /// * `input_fee_rate` - A `Decimal` representing the new fee rate to be applied.
        ///                      The valid range for this rate is between zero and one, where a value of `0.003` equates to a fee rate of 3%.
        ///
        /// # Panics
        /// Panics if the `input_fee_rate` is not valid as determined by `assert_input_fee_rate_is_valid`.
        fn set_input_fee_rate(&mut self, input_fee_rate: Decimal) {
            assert_input_fee_rate_is_valid(input_fee_rate);
            self.input_fee_rate = input_fee_rate;
        }

        /// Sets the protocol fee share for the pool.
        ///
        /// This method updates the `fee_protocol_share` state of the pool. It ensures that the value is within the allowed range [0, `FEE_PROTOCOL_SHARE_MAX`].
        /// The clamping is crucial to prevent setting a fee share that exceeds the maximum allowed limit, which could lead to incorrect fee calculations.
        ///
        /// # Arguments
        /// * `fee_protocol_share` - A `Decimal` representing the new protocol fee share to be set.
        fn set_fee_protocol_share(&mut self, fee_protocol_share: Decimal) {
            self.fee_protocol_share = fee_protocol_share.clamp(dec!(0), FEE_PROTOCOL_SHARE_MAX);
        }

        /// Returns the claimable fees for a given liquidity position.
        ///
        /// This method calculates the fees accrued in a specific liquidity position represented by an NFT.
        /// It updates the fee checkpoints for both x and y tokens based on the current state of the pool and the position's bounds.
        /// The method ensures that the fee calculation is accurate by considering the fees outside the position's range and the current active tick.
        ///
        /// # Arguments
        /// * `position` - A reference to the `LiquidityPosition` representing the liquidity position.
        ///
        /// # Returns
        /// A tuple containing two `Decimal`s:
        /// * The first `Decimal` contains the x token fees claimable.
        /// * The second `Decimal` contains the y token fees claimable.
        ///
        /// # Panics
        /// Panics if the ticks corresponding to the position's bounds are not found in the pool's tick map.
        fn claimable_fees_internal(
            &self,
            position: &LiquidityPosition,
        ) -> (Decimal, Decimal, PreciseDecimal, PreciseDecimal) {
            let left_tick = self.ticks.get(&position.left_bound).unwrap();
            let right_tick = self.ticks.get(&position.right_bound).unwrap();

            let new_x_fee_checkpoint = value_in_range(
                self.x_lp_fee,
                left_tick.x_fee_outside,
                right_tick.x_fee_outside,
                self.active_tick,
                position.left_bound,
                position.right_bound,
            );

            let new_y_fee_checkpoint = value_in_range(
                self.y_lp_fee,
                left_tick.y_fee_outside,
                right_tick.y_fee_outside,
                self.active_tick,
                position.left_bound,
                position.right_bound,
            );

            let x_amount = ((new_x_fee_checkpoint - position.x_fee_checkpoint)
                * position.liquidity)
                .floor_to(self.x_divisibility());
            let y_amount = ((new_y_fee_checkpoint - position.y_fee_checkpoint)
                * position.liquidity)
                .floor_to(self.y_divisibility());

            (
                x_amount,
                y_amount,
                new_x_fee_checkpoint,
                new_y_fee_checkpoint,
            )
        }

        /// Returns the claimable fees for all positions identified by the provided non-fungible local IDs.
        ///
        /// This method calculates all fees (both `x` and `y` tokens) that have accrued to the liquidity positions
        /// specified by the non-fungible local IDs (`lp_position_ids`). It iterates over each position, calculates the fees,
        /// and aggregates them into totals for `x` and `y` fees respectively.
        ///
        /// # Arguments
        /// * `lp_position_ids` - A vector of `NonFungibleLocalId` containing the IDs of liquidity positions for which fees are to be calculated.
        ///
        /// # Returns
        /// - `IndexMap<ResourceAddress, Decimal>` - A map containing the resource addresses and their corresponding total claimable fees.
        pub fn claimable_fees(
            &self,
            lp_position_ids: Vec<NonFungibleLocalId>,
        ) -> IndexMap<ResourceAddress, Decimal> {
            let mut x_fees = dec!(0);
            let mut y_fees = dec!(0);
            for position_id in lp_position_ids {
                let position: LiquidityPosition =
                    self.lp_manager.get_non_fungible_data(&position_id);
                let (x_claimable, y_claimable, _, _) = self.claimable_fees_internal(&position);
                x_fees += x_claimable;
                y_fees += y_claimable;
            }
            IndexMap::from([
                (self.x_fees.resource_address(), x_fees),
                (self.y_fees.resource_address(), y_fees),
            ])
        }

        /// Claims the accumulated fees for a given liquidity position.
        ///
        /// This method calculates and distributes the fees accrued in a specific liquidity position represented by an NFT.
        /// It updates the fee checkpoints for both x and y tokens based on the current state of the pool and the position's bounds.
        /// The method ensures that the fee distribution is accurate by considering the fees outside the position's range and the current active tick.
        ///
        /// # Arguments
        /// * `position_nft` - A reference to the non-fungible token representing the liquidity position.
        ///
        /// # Returns
        /// A tuple containing two `Bucket`s:
        /// * The first `Bucket` contains the x token fees claimed.
        /// * The second `Bucket` contains the y token fees claimed.
        ///
        /// # Panics
        /// Panics if the ticks corresponding to the position's bounds are not found in the pool's tick map.
        fn claim_fees_internal(
            &mut self,
            position_nft: &NonFungible<LiquidityPosition>,
        ) -> (Bucket, Bucket) {
            let position_id: &NonFungibleLocalId = position_nft.local_id();
            let position: LiquidityPosition = position_nft.data();

            let (x_amount, y_amount, new_x_fee_checkpoint, new_y_fee_checkpoint) =
                self.claimable_fees_internal(&position);

            self.lp_manager.update_non_fungible_data(
                position_id,
                "x_fee_checkpoint",
                new_x_fee_checkpoint,
            );
            self.lp_manager.update_non_fungible_data(
                position_id,
                "y_fee_checkpoint",
                new_y_fee_checkpoint,
            );

            Runtime::emit_event(ClaimFeesEvent {
                position_id: position_nft.local_id().clone(),
                position: self.lp_manager.get_non_fungible_data(position_id),
                x_amount,
                y_amount,
            });
            (self.x_fees.take(x_amount), self.y_fees.take(y_amount))
        }

        /// Claims accumulated fees for all positions held in the provided non-fungible proofs.
        ///
        /// This method aggregates all fees (both `x` and `y` tokens) that have accrued to the liquidity positions
        /// specified by the non-fungible proofs (`lp_proofs`). It iterates over each position, claims the fees,
        /// and aggregates them into buckets for `x` and `y` fees respectively.
        ///
        /// # Arguments
        /// * `lp_proofs` - NonFungibleProof containing the proofs of liquidity positions for which fees are to be claimed.
        ///
        /// # Returns
        /// A tuple containing two `Bucket`s:
        /// - The first bucket contains all claimed `x` fees.
        /// - The second bucket contains all claimed `y` fees.
        pub fn claim_fees(&mut self, lp_proofs: NonFungibleProof) -> (Bucket, Bucket) {
            let mut x_fees = Bucket::new(self.x_address());
            let mut y_fees = Bucket::new(self.y_address());
            for position_nft in lp_proofs
                .check(self.lp_manager.address())
                .non_fungibles::<LiquidityPosition>()
            {
                let (x_claimed, y_claimed) = self.claim_fees_internal(&position_nft);
                x_fees.put(x_claimed);
                y_fees.put(y_claimed);
            }
            (x_fees, y_fees)
        }

        /// Calculates the number of seconds a liquidity position has been active being in range.
        ///
        /// # Arguments
        /// * `nft_id` - The identifier for the non-fungible token representing the liquidity position.
        ///
        /// # Returns
        /// * `u64` - The number of seconds the position has been active.
        pub fn seconds_in_position(&mut self, nft_id: NonFungibleLocalId) -> u64 {
            // Retrieve the liquidity position data using the NFT ID.
            let lp_position = self
                .lp_manager
                .get_non_fungible_data::<LiquidityPosition>(&nft_id);

            // Calculate the current active seconds inside the bounds using global pool data and specific tick information.
            let seconds_inside_now = value_in_range(
                self.seconds_global() as i64,
                self.ticks
                    .get(&lp_position.left_bound)
                    .unwrap()
                    .seconds_outside as i64,
                self.ticks
                    .get(&lp_position.right_bound)
                    .unwrap()
                    .seconds_outside as i64,
                self.active_tick,
                lp_position.left_bound,
                lp_position.right_bound,
            );

            // Subtract the checkpoint from the calculated seconds to get the net active seconds since the last checkpoint.
            (seconds_inside_now - lp_position.seconds_inside_checkpoint) as u64
        }

        /// Calculates the total fees accrued for a given liquidity position in both `x` and `y` tokens.
        ///
        /// This method computes the fees by determining the fee checkpoints for both `x` and `y` tokens
        /// based on the current active tick and the bounds of the liquidity position. It then calculates
        /// the difference between the new fee checkpoints and the stored total fee checkpoints of the position,
        /// scaled by the liquidity of the position.
        ///
        /// # Arguments
        /// * `position` - A reference to the `LiquidityPosition` struct representing the liquidity position.
        ///
        /// # Returns
        /// * `(Decimal, Decimal)` - A tuple containing the accrued `x` and `y` fees.
        fn total_fees_internal(&self, position: &LiquidityPosition) -> (Decimal, Decimal) {
            let left_tick = self.ticks.get(&position.left_bound).unwrap();
            let right_tick = self.ticks.get(&position.right_bound).unwrap();

            // Calculate the new fee checkpoints for `x` and `y` tokens within the position's bounds.
            let new_x_fee_checkpoint = value_in_range(
                self.x_lp_fee,
                left_tick.x_fee_outside,
                right_tick.x_fee_outside,
                self.active_tick,
                position.left_bound,
                position.right_bound,
            );

            let new_y_fee_checkpoint = value_in_range(
                self.y_lp_fee,
                left_tick.y_fee_outside,
                right_tick.y_fee_outside,
                self.active_tick,
                position.left_bound,
                position.right_bound,
            );

            // Compute the accrued fees by subtracting the stored total fee checkpoints from the new checkpoints,
            // then multiplying by the position's liquidity.
            let x_amount = ((new_x_fee_checkpoint - position.x_total_fee_checkpoint)
                * position.liquidity)
                .floor_to(self.x_divisibility());
            let y_amount = ((new_y_fee_checkpoint - position.y_total_fee_checkpoint)
                * position.liquidity)
                .floor_to(self.y_divisibility());

            (x_amount, y_amount)
        }

        /// Returns the total fees for all positions identified by the provided non-fungible local IDs.
        ///
        /// This method calculates the total fees (both `x` and `y` tokens) that have accrued to the liquidity positions
        /// specified by the non-fungible local IDs (`lp_position_ids`). It iterates over each position, calculates the fees,
        /// and aggregates them into totals for `x` and `y` fees respectively.
        ///
        /// # Arguments
        /// * `lp_position_ids` - A vector of `NonFungibleLocalId` containing the IDs of liquidity positions for which fees are to be calculated.
        ///
        /// # Returns
        /// - `IndexMap<ResourceAddress, Decimal>` - A map containing the resource addresses and their corresponding total fees.
        pub fn total_fees(
            &self,
            lp_position_ids: Vec<NonFungibleLocalId>,
        ) -> IndexMap<ResourceAddress, Decimal> {
            let mut x_fees = dec!(0);
            let mut y_fees = dec!(0);
            for position_id in lp_position_ids {
                let position: LiquidityPosition =
                    self.lp_manager.get_non_fungible_data(&position_id);
                let (x_total, y_total) = self.total_fees_internal(&position);
                x_fees += x_total;
                y_fees += y_total;
            }
            IndexMap::from([
                (self.x_fees.resource_address(), x_fees),
                (self.y_fees.resource_address(), y_fees),
            ])
        }

        /// Executes a swap, handling deposits and withdrawals based on the swap type.
        ///
        /// This function first aggregates the input tokens with liquidity provider fees and then deposits
        /// protocol fees into a designated vault. It also updates the global liquidity provider fee rate
        /// for the token being bought or sold.
        ///
        /// # Arguments
        /// * `state` - An instance of `SwapState` containing details about the swap, including the type,
        ///   amounts involved, and fees.
        /// * `input_bucket` - A mutable reference to a `Bucket` containing the input tokens for the swap.
        ///
        /// # Returns
        /// * `(Bucket, Bucket)` - A tuple where the first element is the bucket of tokens withdrawn as output
        ///   of the swap, and the second element is the remaining input bucket after fees have been taken.
        fn swap_deposit_and_withdraw(
            &mut self,
            state: SwapState,
            mut input_bucket: Bucket,
        ) -> (Bucket, Bucket) {
            // Take input net and fee from the input bucket which afterwards gets returned as remainder.
            let input_net = input_bucket.take(state.input);
            let fee_lp = input_bucket.take(state.fee_lp_input);
            // Deposit protocol fees into the designated vault.
            self.deposit_protocol_fees(input_bucket.take(state.fee_protocol_input));

            match state.swap_type {
                SwapType::BuyX => {
                    // Withdraw the specified output amount from the `x` liquidity vault.
                    let output = self.x_liquidity.take(state.output);

                    // Deposit the input net and fee tokens into the vaults and update the global fee count.
                    self.y_liquidity.put(input_net);
                    self.y_fees.put(fee_lp);
                    self.y_lp_fee = state.global_input_fee_lp;

                    (output, input_bucket)
                }
                SwapType::SellX => {
                    // Withdraw the specified output amount from the `y` liquidity vault.
                    let output = self.y_liquidity.take(state.output);

                    // Deposit the input net and fee tokens into the vaults and update the global fee count.
                    self.x_liquidity.put(input_net);
                    self.x_fees.put(fee_lp);
                    self.x_lp_fee = state.global_input_fee_lp;

                    (output, input_bucket)
                }
            }
        }

        /// Determines the type of swap based on the input token's address.
        ///
        /// # Arguments
        /// * `input_address` - The address of the input token.
        ///
        /// # Returns
        /// * `SwapType` - The type of the swap (SellX or BuyX).
        fn swap_type(&self, input_address: ResourceAddress) -> SwapType {
            if input_address == self.x_address() {
                return SwapType::SellX;
            }
            SwapType::BuyX
        }

        /// Returns the output token address based on the swap type.
        ///
        /// # Arguments
        /// * `swap_type` - The type of the swap (BuyX or SellX).
        ///
        /// # Returns
        /// * `ResourceAddress` - The address of the output token.
        fn output_address(&self, swap_type: SwapType) -> ResourceAddress {
            match swap_type {
                SwapType::BuyX => self.x_address(),
                SwapType::SellX => self.y_address(),
            }
        }

        /// Retrieves the divisibility of the input token based on the swap type.
        ///
        /// # Arguments
        /// * `swap_type` - The type of the swap (BuyX or SellX).
        ///
        /// # Returns
        /// * `u8` - The divisibility of the input token.
        fn input_divisibility(&self, swap_type: SwapType) -> u8 {
            match swap_type {
                SwapType::BuyX => self.y_divisibility(),
                SwapType::SellX => self.x_divisibility(),
            }
        }

        /// Retrieves the divisibility of the output token based on the swap type.
        ///
        /// # Arguments
        /// * `swap_type` - The type of the swap (BuyX or SellX).
        ///
        /// # Returns
        /// * `u8` - The divisibility of the output token.
        fn output_divisibility(&self, swap_type: SwapType) -> u8 {
            match swap_type {
                SwapType::BuyX => self.x_divisibility(),
                SwapType::SellX => self.y_divisibility(),
            }
        }

        /// Provides the global fees associated with the swap type.
        ///
        /// This method returns a tuple of liquidity provider fees for both tokens involved in the swap.
        /// For BuyX, it returns the fees for token `y` (input) and token `x` (output).
        /// For SellX, it returns the fees for token `x` (input) and token `y` (output).
        ///
        /// # Arguments
        /// * `swap_type` - The type of the swap (BuyX or SellX).
        ///
        /// # Returns
        /// * `(PreciseDecimal, PreciseDecimal)` - The global liquidity provider fees for the input and output tokens.
        fn global_fees(&self, swap_type: SwapType) -> (PreciseDecimal, PreciseDecimal) {
            match swap_type {
                SwapType::BuyX => (self.y_lp_fee, self.x_lp_fee),
                SwapType::SellX => (self.x_lp_fee, self.y_lp_fee),
            }
        }

        /// This should not be used in isolation to judge the merit of a trade,
        /// but be used in combination with the liquidity distribution
        /// along the different price intervals.
        ///
        /// # Returns
        /// * The square root of the current price
        pub fn price_sqrt(&self) -> PreciseDecimal {
            self.price_sqrt
        }

        /// Retrieves the last value of the input fee rate.
        /// The fee rate is static, unless dynamic fee hooks are being used by the pool.
        /// In this case, the value only serves as an indicative,
        /// which should not be relied upon for sensitive matters.
        ///
        /// # Returns
        /// * The last value of the input fee rate
        pub fn input_fee_rate(&self) -> Decimal {
            self.input_fee_rate
        }

        /// Retrieve the current share of the trading fees
        /// that will be awarded to the protocol. This
        /// may change over time, as the pool syncs with the registry.
        ///
        /// # Returns
        /// * The current protocol share of the fees, in fraction form
        pub fn fee_protocol_share(&self) -> Decimal {
            self.fee_protocol_share
        }

        /// Retrieve the flash loan fee rate.
        /// This fee is set during pool instantiation and is immutable.
        ///
        /// # Returns
        /// * The flash loan fee rate
        pub fn flash_loan_fee_rate(&self) -> Decimal {
            self.flash_loan_fee_rate
        }

        /// Retrieve the resource address of the LP token NFTs used in this pool
        ///
        /// # Returns
        /// * The resource address of the LP token NFTs used in this pool
        pub fn lp_address(&self) -> ResourceAddress {
            self.lp_manager.address()
        }

        /// Retrieve the divisibility of the X token in this pool
        ///
        /// # Returns
        /// * The divisibility of the X token
        pub fn x_divisibility(&self) -> u8 {
            self.x_liquidity
                .resource_manager()
                .resource_type()
                .divisibility()
                .unwrap()
        }

        /// Retrieve the divisibility of the Y token in this pool
        ///
        /// # Returns
        /// * The divisibility of the Y token
        pub fn y_divisibility(&self) -> u8 {
            self.y_liquidity
                .resource_manager()
                .resource_type()
                .divisibility()
                .unwrap()
        }

        /// Retrieve the amounts of the X and Y token liquidity in this pool.
        ///
        /// # Returns
        /// * `IndexMap<ResourceAddress, Decimal>` - A map containing the resource addresses and their corresponding amounts.
        pub fn total_liquidity(&self) -> IndexMap<ResourceAddress, Decimal> {
            IndexMap::from([
                (
                    self.x_liquidity.resource_address(),
                    self.x_liquidity.amount(),
                ),
                (
                    self.y_liquidity.resource_address(),
                    self.y_liquidity.amount(),
                ),
            ])
        }

        /// Retrieves the current active tick for the pool.
        ///
        /// The active tick represents the current position of the pool in terms of price levels,
        /// which is always the closest next tick left of the current pool price.
        ///
        /// # Returns
        /// * `Option<i32>` - The active tick index if it is set, or `None` if not.
        pub fn active_tick(&self) -> Option<i32> {
            self.active_tick
        }

        /// Retrieves the resource address for the flash loan NFTs utilized by this pool.
        ///
        /// Flash loan NFTs are used to represent temporary ownership during flash loan operations.
        /// This address is essential for identifying and interacting with the correct NFTs
        /// that facilitate the flash loan mechanism within the pool.
        ///
        /// # Returns
        /// * `ResourceAddress` - The address of the flash loan NFTs used in this pool.
        pub fn flash_loan_address(&self) -> ResourceAddress {
            self.flash_manager.address()
        }

        /// Calculates the elapsed time in seconds since the pool was instantiated.
        ///
        /// # Returns
        /// * `u64` - The number of seconds elapsed since the pool's instantiation.
        fn seconds_global(&mut self) -> u64 {
            Clock::time_in_seconds() - self.instantiated_at
        }

        /// Updates an existing tick or inserts a new one based on the provided parameters.
        ///
        /// This method is crucial for maintaining the state of liquidity at different price levels (ticks) within the pool.
        /// It adjusts the liquidity by adding the `delta_liquidity` to the current liquidity of the tick.
        /// If the tick does not exist, it creates a new tick with the given parameters.
        ///
        /// # Arguments
        /// * `tick_index` - The index of the tick to update or insert.
        /// * `delta_liquidity` - The change in liquidity to apply to the tick.
        /// * `total_liquidity` - The new total liquidity after applying the change.
        ///
        /// # Returns
        /// * `Tick` - The updated or newly created tick.
        ///
        /// # Panics
        /// Panics if the resulting total liquidity exceeds the maximum allowed liquidity per tick.
        fn update_or_insert_tick(
            &mut self,
            tick_index: i32,
            delta_liquidity: PreciseDecimal,
            total_liquidity: PreciseDecimal,
        ) -> Tick {
            if let Some(mut tick) = self.ticks.get_mut(&tick_index) {
                (*tick).delta_liquidity += delta_liquidity;
                (*tick).total_liquidity += total_liquidity;
                assert!(
                    tick.total_liquidity <= self.max_liquidity_per_tick,
                    "Cannot add more total liquidity to the tick than allowed by max_liquidity_per_tick"
                );
                return (*tick).clone();
            }

            let price_sqrt = tick_to_price_sqrt(tick_index);
            let (x_fee_outside, y_fee_outside, seconds_outside) = if self.price_sqrt < price_sqrt {
                (pdec!(0), pdec!(0), 0)
            } else {
                (self.x_lp_fee, self.y_lp_fee, self.seconds_global())
            };

            let tick = Tick {
                index: tick_index,
                delta_liquidity,
                total_liquidity,
                price_sqrt,
                x_fee_outside,
                y_fee_outside,
                seconds_outside,
            };

            assert!(
                tick.total_liquidity <= self.max_liquidity_per_tick,
                "Cannot add more total liquidity to the tick than allowed by max_liquidity_per_tick"
            );

            self.ticks.insert(tick_index, tick.clone());
            tick
        }

        /// Updates or removes a tick based on the liquidity changes.
        ///
        /// This method adjusts the liquidity of a tick and removes the tick if its total liquidity reaches zero.
        /// It is essential for correctly managing the removal of liquidity from the pool and ensuring that empty ticks are cleaned up.
        ///
        /// # Arguments
        /// * `tick_index` - The index of the tick to update or remove.
        /// * `delta_liquidity` - The change in liquidity to apply.
        /// * `total_liquidity` - The new total liquidity after applying the change.
        ///
        /// # Returns
        /// * `Tick` - The updated tick, or the removed tick if its total liquidity is zero.
        fn update_or_remove_tick(
            &mut self,
            tick_index: i32,
            delta_liquidity: PreciseDecimal,
            total_liquidity: PreciseDecimal,
        ) -> Tick {
            let tick = self.update_or_insert_tick(tick_index, delta_liquidity, total_liquidity);
            if tick.total_liquidity == pdec!(0) {
                return self.ticks.remove(&tick_index).unwrap();
            }
            tick
        }

        /// Retrieves an iterator over the ticks for a given swap type.
        ///
        /// This method provides access to the ticks that are relevant for a particular swap operation.
        /// Depending on the swap type, it either includes (in case of `SellX`) or excludes (in case of `BuyX`)
        /// the active tick in the range.
        ///
        /// # Arguments
        /// * `swap_type` - The type of swap operation (`SellX` or `BuyX`).
        ///
        /// # Returns
        /// * `NodeIteratorMut<'_, i32, Tick>` - An iterator over the ticks within the specified range.
        fn get_next_ticks(&mut self, swap_type: SwapType) -> NodeIteratorMut<'_, i32, Tick> {
            match swap_type {
                SwapType::SellX => {
                    // we DO want to include the active tick
                    self.ticks.range_back_mut((
                        Bound::Unbounded,
                        Bound::Included(self.active_tick.unwrap_or(i32::MIN)),
                    ))
                }
                SwapType::BuyX => {
                    // we DONT want to include the active tick
                    self.ticks.range_mut((
                        Bound::Excluded(self.active_tick.unwrap_or(i32::MIN)),
                        Bound::Unbounded,
                    ))
                }
            }
        }

        /// Initiates taking a flash loan from this pool.
        /// Flash loans allow borrowing of funds within a single transaction, provided the loan is repaid by the end
        /// of the transaction with an added fee. This enables users to utilize liquidity for arbitrage, collateral
        /// swapping, or other financial activities without upfront capital.
        ///
        /// # Arguments
        /// * `address`: The `ResourceAddress` of the token to be borrowed.
        /// * `loan_amount`: The amount of the token to be borrowed.
        ///
        /// # Returns
        /// A tuple containing two `Bucket`s:
        /// * The first `Bucket` holds the borrowed amount.
        /// * The second `Bucket` contains a transient non-fungible token (NFT) representing the loan terms, including
        ///   the borrowed token address, the total amount due, and the loan fee.
        pub fn flash_loan(
            &mut self,
            address: ResourceAddress,
            loan_amount: Decimal,
        ) -> (Bucket, Bucket) {
            // Determines the divisibility of the token to ensure the loan amount respects the token's smallest unit.
            let divisibility = ResourceManager::from_address(address)
                .resource_type()
                .divisibility()
                .unwrap();
            let amount = loan_amount.floor_to(divisibility);

            // Withdraws the specified amount from the appropriate liquidity vault.
            let output_bucket = if address == self.x_address() {
                self.x_liquidity.take(amount)
            } else {
                self.y_liquidity.take(amount)
            };

            // Calculates the loan fee based on the specified rate and adds it to the borrowed amount to determine the total amount due.
            let fee =
                (PreciseDecimal::from(amount) * self.flash_loan_fee_rate).ceil_to(divisibility);

            let flash_loan = FlashLoan {
                address,
                due_amount: amount + fee,
                fee,
            };

            Runtime::emit_event(FlashLoanEvent {
                address: flash_loan.address,
                due_amount: flash_loan.due_amount,
                fee: flash_loan.fee,
            });

            // Mints a transient NFT that encapsulates the terms of the loan for use in repayment validation.
            let transient_loan_bucket = self.flash_manager.mint_ruid_non_fungible(flash_loan);

            (output_bucket, transient_loan_bucket)
        }

        /// Repays a flash loan, ensuring the correct amount and token type are returned, and handles the protocol fees.
        ///
        /// This method is called to repay a flash loan. It validates the repayment bucket and the loan terms,
        /// ensures the repayment amount is sufficient, and separates the protocol fees from the principal repayment.
        /// The method concludes by burning the loan terms NFT to signify the loan closure.
        ///
        /// # Arguments
        /// * `loan_repayment`: A bucket containing the tokens used to repay the loan.
        /// * `loan_terms`: A non-fungible token (NFT) representing the terms of the loan.
        ///
        /// # Returns
        /// * A bucket containing any excess tokens after the loan and fees have been repaid.
        ///
        /// # Panics
        /// * If the `loan_terms` NFT does not belong to the flash manager's address.
        /// * If the token type in `loan_repayment` does not match the token type in the loan terms.
        /// * If the amount in `loan_repayment` is less than the due amount specified in the loan terms.
        pub fn repay_loan(
            &mut self,
            mut loan_repayment: Bucket,
            loan_terms: NonFungibleBucket,
        ) -> Bucket {
            // Ensure the loan terms NFT is from the correct resource manager.
            assert!(
                loan_terms.resource_address() == self.flash_manager.address(),
                "Incorrect resource passed in for loan terms"
            );

            // Retrieve the loan terms from the NFT.
            // Important here is `non_fungible` panics if there are more than one NFTs in the bucket
            // which would allow to fully drain the pool
            let transient = loan_terms.non_fungible::<FlashLoan>();
            let terms: FlashLoan = transient.data();

            // Validate that the repayment is being made with the correct token type.
            assert!(
                terms.address == loan_repayment.as_fungible().resource_address(),
                "Incorrect resource to repay loan"
            );

            // Ensure the repayment amount meets or exceeds the due amount.
            assert!(
                loan_repayment.amount() >= terms.due_amount,
                "Insufficient repayment given for your loan!"
            );

            // Separate the fee from the repayment amount and deposit it as protocol fees.
            self.deposit_protocol_fees(loan_repayment.take(terms.fee));

            // Calculate the principal amount to be returned to the appropriate vault.
            let loan_amount = terms.due_amount - terms.fee;

            // Return the principal amount to the correct vault based on the token address.
            if terms.address == self.x_address() {
                self.x_liquidity.put(loan_repayment.take(loan_amount));
            } else {
                self.y_liquidity.put(loan_repayment.take(loan_amount));
            }

            // Burn the loan terms NFT to officially close the loan.
            self.flash_manager.burn(loan_terms);

            // Return any excess tokens to the caller.
            loan_repayment
        }

        /// Deposits the collected protocol fees into the appropriate vault.
        ///
        /// This method determines the type of token in the `protocol_fees` bucket and deposits it into the corresponding
        /// vault (`x_protocol_fee` or `y_protocol_fee`).
        ///
        /// # Arguments
        /// * `protocol_fees`: A `Bucket` containing the fees collected during pool operations that need to be deposited as protocol fees.
        fn deposit_protocol_fees(&mut self, protocol_fees: Bucket) {
            if protocol_fees.resource_address() == self.x_address() {
                self.x_protocol_fee.put(protocol_fees);
            } else {
                self.y_protocol_fee.put(protocol_fees);
            }
        }

        /// Executes predefined hooks based on the lifecycle event of the pool.
        ///
        /// This method applies custom logic at different stages of the pool's lifecycle,
        /// like before or after swaps, and during liquidity changes or initialization.
        /// It uses hooks to implement modular, event-driven logic that can be customized and linked to these events.
        ///
        /// # Arguments
        /// * `hook_call` - An enum representing the specific lifecycle event.
        /// * `hook_args` - The arguments to pass to the hook functions, allowing for context-specific actions.
        ///
        /// # Returns
        /// Returns the modified hook arguments after all relevant hooks have been executed,
        /// which may carry state changes enacted by the hooks.
        fn execute_hooks<T: ScryptoSbor>(&self, hook_call: HookCall, hook_args: T) -> T {
            let hooks = match hook_call {
                HookCall::BeforeInstantiate => &self.hook_calls.before_instantiate,
                HookCall::AfterInstantiate => &self.hook_calls.after_instantiate,
                HookCall::BeforeSwap => &self.hook_calls.before_swap,
                HookCall::AfterSwap => &self.hook_calls.after_swap,
                HookCall::BeforeAddLiquidity => &self.hook_calls.before_add_liquidity,
                HookCall::AfterAddLiquidity => &self.hook_calls.after_add_liquidity,
                HookCall::BeforeRemoveLiquidity => &self.hook_calls.before_remove_liquidity,
                HookCall::AfterRemoveLiquidity => &self.hook_calls.after_remove_liquidity,
            };
            execute_hooks(&hooks, &self.hook_badges, hook_args)
        }

        /// Retrieves a registered hook component based on its package address and blueprint name.
        ///
        /// This method is crucial for the dynamic invocation of specific functionalities encapsulated within
        /// different components of the system. By providing a package address and a blueprint name, it allows
        /// for the retrieval of the corresponding component if it has been previously registered in the `hooks` map.
        /// This is particularly useful for extending or modifying behavior at runtime without altering the underlying
        /// blueprint code.
        ///
        /// # Arguments
        /// * `package_address` - The address of the package where the component is defined.
        /// * `blueprint_name` - The name of the blueprint within the specified package.
        ///
        /// # Returns
        /// An `Option<Global<AnyComponent>>` which is:
        /// - `Some(Global<AnyComponent>)` if the hook is found, allowing further interaction with the component.
        /// - `None` if no such hook is registered, indicating the absence of the component under the specified identifiers.
        pub fn hook(
            &self,
            package_address: PackageAddress,
            blueprint_name: String,
        ) -> Option<Global<AnyComponent>> {
            self.hooks
                .get(&(package_address, blueprint_name))
                .map(|hook| hook.to_owned())
        }

        /// Generates names and descriptions for the pool and LP tokens.
        ///
        /// This function constructs the names and descriptions for the pool and its associated LP tokens
        /// based on the symbols of the provided resource addresses.
        ///
        /// # Arguments
        /// * `x_address` - The resource address of the first asset in the pool.
        /// * `y_address` - The resource address of the second asset in the pool.
        ///
        /// # Returns
        /// A tuple containing:
        /// - `pool_name`: The name of the pool.
        /// - `lp_name`: The name of the LP token.
        /// - `lp_description`: The description of the LP token.
        fn names_and_lp_description(
            x_address: ResourceAddress,
            y_address: ResourceAddress,
        ) -> (String, String, String) {
            let x_symbol = token_symbol(x_address);
            let y_symbol = token_symbol(y_address);
            let (pool_name, lp_name, lp_description) =
                match x_symbol.zip(y_symbol).map(|(x, y)| format!("{}/{}", x, y)) {
                    Some(pair_symbol) => (
                        format!("Ociswap Precision Pool {}", pair_symbol).to_owned(),
                        format!("Ociswap LP {}", pair_symbol).to_owned(),
                        format!("Ociswap LP token for Precision Pool {}", pair_symbol).to_owned(),
                    ),
                    None => (
                        "Ociswap Precision Pool".to_owned(),
                        "Ociswap LP".to_owned(),
                        "Ociswap LP token for Precision Pool".to_owned(),
                    ),
                };
            (pool_name, lp_name, lp_description)
        }

        /// Sets up a liquidity position manager for a specific pool.
        ///
        /// This function initializes a non-fungible resource manager for liquidity positions in a pool,
        /// which represents the ownership of liquidity in the pool. It assigns metadata, minting, burning,
        /// and data update roles specific to the pool's operational requirements.
        ///
        /// # Arguments
        /// * `pool_address` - The address of the pool for which the LP manager is being set.
        /// * `x_address` - The resource address of the first asset in the pool.
        /// * `y_address` - The resource address of the second asset in the pool.
        /// * `dapp_definition` - The address of the dApp definition component.
        ///
        /// # Returns
        /// A `ResourceManager` that can manage the non-fungible tokens representing liquidity positions.
        fn set_lp_manager(
            pool_address: ComponentAddress,
            name: String,
            description: String,
            dapp_definition: ComponentAddress,
        ) -> ResourceManager {
            let tags = vec![
                "ociswap".to_owned(),
                "liquidity-pool".to_owned(),
                "precision-pool".to_owned(),
                "lp".to_owned(),
                "dex".to_owned(),
                "defi".to_owned(),
            ];
            let dapp_definition_global: GlobalAddress = dapp_definition.into();
            let pool_address_global: GlobalAddress = pool_address.into();

            let lp_manager = ResourceBuilder::new_integer_non_fungible::<LiquidityPosition>(
                OwnerRole::None
            )
                .metadata(
                    metadata! {
                        init {
                            "name" => name, locked;
                            "description" => description, locked;
                            "tags" => tags.to_owned(), locked;
                            "icon_url" => Url::of("https://ociswap.com/icons/lp_token.png".to_owned()), locked;
                            "info_url" => Url::of(format!("https://ociswap.com/pools/{}", Runtime::bech32_encode_address(pool_address)).to_owned()), locked;
                            "pool" => pool_address_global, locked;
                            "dapp_definition" => dapp_definition_global, locked;
                        }
                    }
                )
                .mint_roles(
                    mint_roles!(
                        minter => rule!(require(global_caller(pool_address)));
                        minter_updater => rule!(deny_all);
                    )
                )
                .burn_roles(
                    burn_roles!(
                        burner => rule!(require(global_caller(pool_address)));
                        burner_updater => rule!(deny_all);
                    )
                )
                .non_fungible_data_update_roles(
                    non_fungible_data_update_roles! {
                        non_fungible_data_updater => rule!(require(global_caller(pool_address)));
                        non_fungible_data_updater_updater => rule!(deny_all);
                    }
                )
                .create_with_no_initial_supply();
            lp_manager
        }

        // ORACLE

        /// Fetches an `AccumulatedObservation` for a specified timestamp.
        ///
        /// This method is crucial for providing accurate and timely price data to the pool's trading operations.
        /// It handles different scenarios based on the provided timestamp:
        ///
        /// - **Existing Observation**: Directly returns the observation if it matches the provided timestamp.
        /// - **Interpolation Needed**: If the timestamp falls between two stored observations, it computes an interpolated observation using the closest available data points.
        /// - **Recent Timestamp**: Generates a new observation if the timestamp is more recent than the latest stored but still within the current time bounds.
        /// - **Out of Bounds**: Triggers a panic for timestamps that are out of the valid range, as they cannot be reliably processed.
        pub fn observation(&self, timestamp: u64) -> AccumulatedObservation {
            self.oracle.observation(timestamp)
        }

        /// Calculates the average price square root over specified time intervals.
        ///
        /// This method is essential for determining the geometric mean of the price square root (`price_sqrt`)
        /// between pairs of timestamps. Each pair in the vector represents a start and end timestamp, defining
        /// an interval for which the average `price_sqrt` is computed. This computation is crucial for financial
        /// analyses and operations that require historical price data over specific periods.
        ///
        /// # Arguments
        /// * `intervals` - A vector of tuples where each tuple contains two Unix timestamps (u64). The first element
        ///   is the start timestamp, and the second is the end timestamp of the interval.
        ///
        /// # Returns
        /// A vector of `ObservationInterval` structs, each representing the average `price_sqrt` over the given interval.
        ///
        /// # Example
        /// ```
        /// let intervals = vec![(1609459200, 1609545600), (1609545600, 1609632000)];
        /// let observation_intervals = pool.observation_intervals(intervals);
        /// ```
        pub fn observation_intervals(
            &self,
            intervals: Vec<(u64, u64)>, // In Unix seconds
        ) -> Vec<ObservationInterval> {
            self.oracle.observation_intervals(intervals)
        }

        /// Returns the maximum number of observations that the oracle can store.
        ///
        /// # Returns
        /// A `u16` representing the maximum number of observations that can be stored.
        pub fn observations_limit(&self) -> u16 {
            self.oracle.observations_limit()
        }

        /// Returns the current number of observations stored in the oracle.
        ///
        /// # Returns
        /// A `u16` representing the current number of observations stored.
        pub fn observations_stored(&self) -> u16 {
            self.oracle.observations_stored()
        }

        /// Returns the timestamp of the oldest observation stored in the oracle.
        ///
        /// # Returns
        /// An `Option<u64>` representing the timestamp of the oldest observation if it exists, or `None` if no observations have been stored yet.
        pub fn oldest_observation_at(&self) -> Option<u64> {
            self.oracle.oldest_observation_at()
        }

        /// Returns the index of the most recent observation stored in the oracle (for testing).
        ///
        /// # Returns
        /// An `Option<u16>` representing the index of the last observation if it exists, or `None` if no observations have been stored yet.
        pub fn last_observation_index(&self) -> Option<u16> {
            self.oracle.last_observation_index()
        }
    }
}

#[derive(ScryptoSbor, NonFungibleData, Clone)]
pub struct LiquidityPosition {
    pub liquidity: PreciseDecimal,
    pub left_bound: i32,
    pub right_bound: i32,
    pub shape_id: Option<NonFungibleLocalId>,
    pub added_at: u64,
    #[mutable]
    x_fee_checkpoint: PreciseDecimal,
    #[mutable]
    y_fee_checkpoint: PreciseDecimal,
    x_total_fee_checkpoint: PreciseDecimal,
    y_total_fee_checkpoint: PreciseDecimal,
    seconds_inside_checkpoint: i64,
}

#[derive(ScryptoSbor, Clone, Debug)]
pub struct Tick {
    pub index: i32,
    pub delta_liquidity: PreciseDecimal,
    pub total_liquidity: PreciseDecimal,
    pub price_sqrt: PreciseDecimal,
    pub x_fee_outside: PreciseDecimal,
    pub y_fee_outside: PreciseDecimal,
    pub seconds_outside: u64,
}

#[derive(ScryptoSbor, Clone, Debug)]
pub struct TickOutside {
    pub index: i32,
    pub x_fee: PreciseDecimal,
    pub y_fee: PreciseDecimal,
    pub seconds: u64,
}

impl Tick {
    /// Returns the fees accumulated outside the current tick for a given swap type.
    ///
    /// Returns accumulated fees outside the current tick's range based on the swap type.
    /// The returned tuple contains fees for the bought and sold tokens respectively.
    ///
    /// # Arguments
    /// * `swap_type` - The type of swap (BuyX or SellX) which determines the direction of trade.
    ///
    /// # Returns
    /// A tuple containing the fee amounts for the tokens involved in the swap outside the current tick.
    pub fn fee_outside(&self, swap_type: SwapType) -> (PreciseDecimal, PreciseDecimal) {
        match swap_type {
            SwapType::BuyX => (self.y_fee_outside, self.x_fee_outside),
            SwapType::SellX => (self.x_fee_outside, self.y_fee_outside),
        }
    }

    /// Updates the tick's outside fee and time values based on global accumulations.
    ///
    /// Updates the tick's outside fee values and time by subtracting the stored values from the global accumulations.
    ///
    /// # Arguments
    /// * `fee_x_global` - The global accumulated fee for token X.
    /// * `fee_y_global` - The global accumulated fee for token Y.
    /// * `seconds_global` - The global accumulated seconds outside this tick.
    ///
    /// # Returns
    /// A `TickOutside` instance containing the updated outside values for this tick.
    pub fn update_outside_values(
        &mut self,
        fee_x_global: PreciseDecimal,
        fee_y_global: PreciseDecimal,
        seconds_global: u64,
    ) -> TickOutside {
        self.x_fee_outside = fee_x_global - self.x_fee_outside;
        self.y_fee_outside = fee_y_global - self.y_fee_outside;
        self.seconds_outside = seconds_global - self.seconds_outside;
        TickOutside {
            index: self.index,
            x_fee: self.x_fee_outside,
            y_fee: self.y_fee_outside,
            seconds: self.seconds_outside,
        }
    }
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct InstantiateEvent {
    pool_address: ComponentAddress,
    lp_address: ResourceAddress,
    x_address: ResourceAddress,
    y_address: ResourceAddress,
    price_sqrt: PreciseDecimal,
    tick_spacing: u32,
    input_fee_rate: Decimal,
    flash_loan_address: ResourceAddress,
    flash_loan_fee_rate: Decimal,
    registry_address: ComponentAddress,
    hooks: Vec<ComponentAddress>,
    dapp_definition: ComponentAddress,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct AddLiquidityEvent {
    position_id: NonFungibleLocalId,
    position: LiquidityPosition,
    left_tick: Tick,
    right_tick: Tick,
    x_amount: Decimal,
    y_amount: Decimal,
    x_gross_amount: Decimal,
    y_gross_amount: Decimal,
    active_liquidity: PreciseDecimal,
    active_tick: Option<i32>,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct RemoveLiquidityEvent {
    position_id: NonFungibleLocalId,
    position: LiquidityPosition,
    left_tick: Tick,
    right_tick: Tick,
    x_amount: Decimal,
    y_amount: Decimal,
    x_return_amount: Decimal,
    y_return_amount: Decimal,
    active_liquidity: PreciseDecimal,
    active_tick: Option<i32>,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct ClaimFeesEvent {
    position_id: NonFungibleLocalId,
    position: LiquidityPosition,
    x_amount: Decimal,
    y_amount: Decimal,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct SwapEvent {
    input_address: ResourceAddress,
    input_amount: Decimal,
    input_gross_amount: Decimal,
    input_fee_lp: Decimal,
    input_fee_protocol: Decimal,
    output_address: ResourceAddress,
    output_amount: Decimal,
    output_return_amount: Decimal,
    price_sqrt: PreciseDecimal,
    active_liquidity: PreciseDecimal,
    active_tick: Option<i32>,
    global_x_fee_lp: PreciseDecimal,
    global_y_fee_lp: PreciseDecimal,
    crossed_ticks: Vec<TickOutside>,
}

#[derive(ScryptoSbor, ScryptoEvent)]
struct FlashLoanEvent {
    address: ResourceAddress,
    due_amount: Decimal,
    fee: Decimal,
}

#[derive(ScryptoSbor, NonFungibleData)]
pub struct FlashLoan {
    pub address: ResourceAddress,
    pub due_amount: Decimal,
    pub fee: Decimal,
}
