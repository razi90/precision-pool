use scrypto::prelude::*;

use common::pools::SwapType;

#[derive(ScryptoSbor, Clone, Debug)]
pub struct BeforeInstantiateState {
    pub price_sqrt: Option<PreciseDecimal>,
    pub x_address: ResourceAddress,
    pub y_address: ResourceAddress,
    pub input_fee_rate: Decimal,
    pub flash_loan_fee_rate: Decimal,
}

#[derive(ScryptoSbor, ManifestSbor, Clone, Debug, PartialEq)]
pub struct AfterInstantiateState {
    pub pool_address: ComponentAddress,
    pub price_sqrt: Option<PreciseDecimal>,
    pub x_address: ResourceAddress,
    pub y_address: ResourceAddress,
    pub input_fee_rate: Decimal,
    pub flash_loan_fee_rate: Decimal,
}

#[derive(ScryptoSbor, Clone, Debug)]
pub struct LiquidityPositionType {
    pub left_bound: i32,
    pub right_bound: i32,
    pub position_id: Option<NonFungibleLocalId>,
    pub shape_id: Option<NonFungibleLocalId>,
}

#[derive(ScryptoSbor, Clone, Debug)]
pub struct BeforeAddLiquidityState {
    pub pool_address: ComponentAddress,
    pub x_provided: Decimal,
    pub y_provided: Decimal,
    pub active_liquidity: PreciseDecimal,
    pub price_sqrt: PreciseDecimal,
    pub position: LiquidityPositionType,
}

#[derive(ScryptoSbor, Clone, Debug)]
pub struct AfterAddLiquidityState {
    pub pool_address: ComponentAddress,
    pub x_added: Decimal,
    pub y_added: Decimal,
    pub added_liquidity: PreciseDecimal,
    pub active_liquidity: PreciseDecimal,
    pub price_sqrt: PreciseDecimal,
    pub position: LiquidityPositionType,
}

#[derive(ScryptoSbor, Clone, Debug)]
pub struct BeforeRemoveLiquidityState {
    pub pool_address: ComponentAddress,
    pub provided_liquidity: PreciseDecimal,
    pub active_liquidity: PreciseDecimal,
    pub price_sqrt: PreciseDecimal,
    pub position: LiquidityPositionType,
}

#[derive(ScryptoSbor, Clone, Debug)]
pub struct AfterRemoveLiquidityState {
    pub pool_address: ComponentAddress,
    pub x_removed: Decimal,
    pub y_removed: Decimal,
    pub removed_liquidity: PreciseDecimal,
    pub active_liquidity: PreciseDecimal,
    pub price_sqrt: PreciseDecimal,
    pub position: LiquidityPositionType,
}

#[derive(ScryptoSbor, ManifestSbor, Clone, Debug, PartialEq)]
pub struct BeforeSwapState {
    pub pool_address: ComponentAddress,
    pub swap_type: SwapType,
    pub price_sqrt: PreciseDecimal,
    pub active_liquidity: PreciseDecimal,
    pub input_fee_rate: Decimal,
    pub fee_protocol_share: Decimal,
}

#[derive(ScryptoSbor, ManifestSbor, Clone, Debug, PartialEq)]
pub struct AfterSwapState {
    pub pool_address: ComponentAddress,
    pub swap_type: SwapType,
    pub price_sqrt: PreciseDecimal,
    pub active_liquidity: PreciseDecimal,
    pub input_fee_rate: Decimal,
    pub fee_protocol_share: Decimal,
    pub input_address: ResourceAddress,
    pub input_amount: Decimal,
    pub output_address: ResourceAddress,
    pub output_amount: Decimal,
    pub input_fee_lp: Decimal,
    pub input_fee_protocol: Decimal,
}

#[derive(ScryptoSbor, Clone, Debug, PartialEq, ManifestSbor)]
pub enum HookCall {
    BeforeInstantiate,
    AfterInstantiate,
    BeforeSwap,
    AfterSwap,
    BeforeAddLiquidity,
    AfterAddLiquidity,
    BeforeRemoveLiquidity,
    AfterRemoveLiquidity,
}

#[derive(ScryptoSbor, Clone, Debug)]
pub struct HookCalls {
    pub before_instantiate: (String, Vec<Global<AnyComponent>>),
    pub after_instantiate: (String, Vec<Global<AnyComponent>>),
    pub before_add_liquidity: (String, Vec<Global<AnyComponent>>),
    pub after_add_liquidity: (String, Vec<Global<AnyComponent>>),
    pub before_swap: (String, Vec<Global<AnyComponent>>),
    pub after_swap: (String, Vec<Global<AnyComponent>>),
    pub before_remove_liquidity: (String, Vec<Global<AnyComponent>>),
    pub after_remove_liquidity: (String, Vec<Global<AnyComponent>>),
}

impl HookCalls {
    fn new() -> Self {
        HookCalls {
            before_instantiate: ("before_instantiate".into(), Vec::new()),
            after_instantiate: ("after_instantiate".into(), Vec::new()),
            before_add_liquidity: ("before_add_liquidity".into(), Vec::new()),
            after_add_liquidity: ("after_add_liquidity".into(), Vec::new()),
            before_swap: ("before_swap".into(), Vec::new()),
            after_swap: ("after_swap".into(), Vec::new()),
            before_remove_liquidity: ("before_remove_liquidity".into(), Vec::new()),
            after_remove_liquidity: ("after_remove_liquidity".into(), Vec::new()),
        }
    }
}

/// Generates hook calls and organizes them by their respective hook types.
///
/// This function takes a list of component addresses paired with their respective badges and:
/// - Maps each hook to its corresponding lifecycle event using `generate_hooks_badges`.
/// - Collects all hooks into a hashmap for quick access using `generate_hooks`.
///
/// ## Arguments
/// - `hook_badges`: A vector of tuples containing component addresses and their associated buckets.
///
/// ## Returns
/// - A tuple containing:
///   - `HookCalls`: Struct containing organized hooks by lifecycle events.
///   - `HashMap<ComponentAddress, Bucket>`: A hashmap linking component addresses to their badges.
///   - `HashMap<(PackageAddress, String), Global<AnyComponent>>`: A hashmap for quick hook access.
pub fn generate_calls_hooks(
    hook_badges: Vec<(ComponentAddress, Bucket)>,
) -> (
    HookCalls,
    HashMap<ComponentAddress, Bucket>,
    HashMap<(PackageAddress, String), Global<AnyComponent>>,
) {
    let (hook_calls, hook_badge_bucket) = generate_hooks_badges(hook_badges);
    let hooks = generate_hooks(&hook_badge_bucket);
    (hook_calls, hook_badge_bucket, hooks)
}

/// Generates hook calls and organizes them into `HookCalls` based on their lifecycle events.
///
/// Iterates over each hook badge pair, retrieves the lifecycle events associated with each hook,
/// and categorizes them into the appropriate lifecycle event in `HookCalls`.
///
/// ## Arguments
/// - `hook_badges`: A vector of tuples containing component addresses and their associated buckets.
///
/// ## Returns
/// - A tuple containing:
///   - `HookCalls`: Struct containing organized hooks by lifecycle events.
///   - `HashMap<ComponentAddress, Bucket>`: A hashmap linking component addresses to their badges.
fn generate_hooks_badges(
    hook_badges: Vec<(ComponentAddress, Bucket)>,
) -> (HookCalls, HashMap<ComponentAddress, Bucket>) {
    let mut hook_calls = HookCalls::new();

    let mut hook_badge_bucket = HashMap::new();

    for (hook_address, badge_bucket) in hook_badges {
        let hook: Global<AnyComponent> = hook_address.into();
        let calls = hook.call_raw::<Vec<HookCall>>("calls", scrypto_args!());

        for call in calls {
            match call {
                HookCall::BeforeInstantiate => hook_calls.before_instantiate.1.push(hook),
                HookCall::AfterInstantiate => hook_calls.after_instantiate.1.push(hook),
                HookCall::BeforeSwap => hook_calls.before_swap.1.push(hook),
                HookCall::AfterSwap => hook_calls.after_swap.1.push(hook),
                HookCall::BeforeAddLiquidity => hook_calls.before_add_liquidity.1.push(hook),
                HookCall::AfterAddLiquidity => hook_calls.after_add_liquidity.1.push(hook),
                HookCall::BeforeRemoveLiquidity => hook_calls.before_remove_liquidity.1.push(hook),
                HookCall::AfterRemoveLiquidity => hook_calls.after_remove_liquidity.1.push(hook),
            }
        }

        hook_badge_bucket.insert(hook_address, badge_bucket);
    }

    (hook_calls, hook_badge_bucket)
}

/// Generates a hashmap for quick access to hooks based on their package and blueprint names.
///
/// Converts each component address in `hook_badge_bucket` to a `Global<AnyComponent>` and maps it
/// to its package and blueprint name for quick lookup.
///
/// ## Arguments
/// - `hook_badge_bucket`: A reference to a hashmap linking component addresses to their badges.
///
/// ## Returns
/// - `HashMap<(PackageAddress, String), Global<AnyComponent>>`: A hashmap for quick hook access.
fn generate_hooks(
    hook_badge_bucket: &HashMap<ComponentAddress, Bucket>,
) -> HashMap<(PackageAddress, String), Global<AnyComponent>> {
    hook_badge_bucket
        .keys()
        .map(|hook| {
            let global: Global<AnyComponent> = (*hook).into();
            (
                (
                    global.blueprint_id().package_address.clone(),
                    global.blueprint_id().blueprint_name.clone(),
                ),
                global,
            )
        })
        .collect()
}

/// Executes hooks for a specific lifecycle event, modifying the input based on the hook's logic.
///
/// Iterates over each hook in the provided lifecycle event, creates a proof of badge ownership,
/// and calls the hook with the input. The input is modified by each hook according to its logic.
///
/// ## Arguments
/// - `hooks`: A tuple containing the lifecycle event name and associated hooks.
/// - `badges`: A hashmap linking component addresses to their vaults containing badges.
/// - `input`: The input to be modified by the hooks.
///
/// ## Returns
/// - `T`: The modified input after all hooks have been executed.
pub fn execute_hooks<T: ScryptoSbor>(
    hooks: &(String, Vec<Global<AnyComponent>>),
    badges: &HashMap<ComponentAddress, Vault>,
    mut input: T,
) -> T {
    for hook in hooks.1.iter() {
        let badge_vault = badges.get(&hook.address()).unwrap();
        let badge_proof = badge_vault.as_fungible().create_proof_of_amount(1);
        LocalAuthZone::push(badge_proof);
        input = hook.call::<T, T>(hooks.0.as_str(), &input);
        LocalAuthZone::drop_proofs();
    }
    input
}

/// Executes hooks specifically before the instantiation of a component, modifying the input.
///
/// Similar to `execute_hooks`, but specifically tailored for the `before_instantiate` lifecycle event.
/// It uses buckets instead of vaults for badge management.
///
/// ## Arguments
/// - `hooks`: A tuple containing the lifecycle event name and associated hooks.
/// - `badges`: A hashmap linking component addresses to their buckets containing badges.
/// - `input`: The input to be modified by the hooks.
///
/// ## Returns
/// - `T`: The modified input after all hooks have been executed.
pub fn execute_hooks_before_instantiate<T: ScryptoSbor>(
    hooks: &(String, Vec<Global<AnyComponent>>),
    badges: &HashMap<ComponentAddress, Bucket>,
    mut input: T,
) -> T {
    for hook in hooks.1.iter() {
        let badge_bucket = badges.get(&hook.address()).unwrap();
        let badge_proof = badge_bucket.as_fungible().create_proof_of_amount(1);
        LocalAuthZone::push(badge_proof);
        input = hook.call::<T, T>(hooks.0.as_str(), &input);
        LocalAuthZone::drop_proofs();
    }
    input
}
