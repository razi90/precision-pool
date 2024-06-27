use common::pools::SwapType;
use precision_pool_hooks::*;
use scrypto::prelude::*;

#[blueprint]
mod test_hook {
    enable_method_auth! {
        roles {
            hook_admin => updatable_by: [OWNER];
        },
        methods {
            calls => PUBLIC;
            before_instantiate => restrict_to: [hook_admin];
            after_instantiate => restrict_to: [hook_admin];
            before_swap => restrict_to: [hook_admin];
            after_swap => restrict_to: [hook_admin];
            before_add_liquidity => restrict_to: [hook_admin];
            after_add_liquidity => restrict_to: [hook_admin];
            before_remove_liquidity => restrict_to: [hook_admin];
            after_remove_liquidity => restrict_to: [hook_admin];
        }
    }
    struct TestHook {
        calls: Vec<HookCall>,
        calls_access: TestAccess,
        x_vault: Vault,
        y_vault: Vault,
    }

    impl TestHook {
        pub fn instantiate(
            calls: Vec<HookCall>,
            calls_access: TestAccess,
            x_address: ResourceAddress,
            y_address: ResourceAddress,
        ) -> (Global<TestHook>, FungibleBucket) {
            let hook_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(DIVISIBILITY_NONE)
                .metadata(metadata! {
                init {
                    "name" => "Hook badge", locked;
                }})
                .mint_roles(mint_roles!(
                    minter => rule!(allow_all);
                    minter_updater => rule!(deny_all);
                ))
                .mint_initial_supply(1);

            debug!("{:?}", hook_badge.resource_address());
            let hook_badge_address = hook_badge.resource_address();

            let x_vault: Vault = Vault::new(x_address);
            let y_vault: Vault = Vault::new(y_address);

            let hook_component = (Self {
                calls,
                calls_access,
                x_vault,
                y_vault,
            })
            .instantiate()
            .prepare_to_globalize(OwnerRole::None)
            .roles(roles!(
                hook_admin => rule!(require(hook_badge_address));
            ))
            .globalize();

            (hook_component, hook_badge)
        }

        pub fn calls(&mut self) -> Vec<HookCall> {
            self.calls.clone()
        }

        pub fn before_instantiate(
            &mut self,
            state: BeforeInstantiateState,
        ) -> (BeforeInstantiateState,) {
            debug!(
                "[TEST HOOK] before_instantiate, price now: {:?} {:?} {:?}",
                state.price_sqrt, state.x_address, state.y_address
            );
            (state,)
        }

        pub fn after_instantiate(
            &mut self,
            state: AfterInstantiateState,
        ) -> (AfterInstantiateState,) {
            debug!(
                "[TEST HOOK] after_instantiate, price: {:?}",
                state.price_sqrt
            );
            (state,)
        }

        pub fn before_swap(
            &mut self,
            swap_state: BeforeSwapState,
            mut input_bucket: Bucket,
        ) -> (BeforeSwapState, Bucket) {
            debug!("[TEST HOOK] before_swap");
            if self.calls_access.before_swap_input {
                debug!("[TEST HOOK] before_swap: In before_swap_input");
                match swap_state.swap_type {
                    SwapType::BuyX => {
                        self.y_vault.put(input_bucket.take(Decimal::ZERO));
                    }
                    SwapType::SellX => {
                        self.x_vault.put(input_bucket.take(Decimal::ZERO));
                    }
                }
            }
            (swap_state, input_bucket)
        }

        pub fn after_swap(
            &mut self,
            swap_state: AfterSwapState,
            mut output_bucket: Bucket,
        ) -> (AfterSwapState, Bucket) {
            debug!("[TEST HOOK] after_swap");
            if self.calls_access.after_swap_output {
                debug!("[TEST HOOK] after_swap: In after_swap_output");
                match swap_state.swap_type {
                    SwapType::BuyX => {
                        self.x_vault.put(output_bucket.take(Decimal::ZERO));
                    }
                    SwapType::SellX => {
                        self.y_vault.put(output_bucket.take(Decimal::ZERO));
                    }
                }
            }
            (swap_state, output_bucket)
        }

        pub fn before_add_liquidity(
            &mut self,
            add_liquidity_state: BeforeAddLiquidityState,
            mut x_bucket: Bucket,
            mut y_bucket: Bucket,
        ) -> (BeforeAddLiquidityState, Bucket, Bucket) {
            debug!("[TEST HOOK] before_add_liquidity");
            if self.calls_access.before_add_liquidity_x_input {
                debug!("[TEST HOOK] before_add_liquidity: In before_add_liquidity_x_input");
                self.x_vault.put(x_bucket.take(Decimal::ZERO));
            }
            if self.calls_access.before_add_liquidity_y_input {
                debug!("[TEST HOOK] before_add_liquidity: In before_add_liquidity_y_input");
                self.y_vault.put(y_bucket.take(Decimal::ZERO));
            }
            (add_liquidity_state, x_bucket, y_bucket)
        }

        pub fn after_add_liquidity(
            &mut self,
            add_liquidity_state: AfterAddLiquidityState,
            lp_token: Bucket,
        ) -> (AfterAddLiquidityState, Bucket) {
            debug!("[TEST HOOK] after_add_liquidity");
            let _liquidity_position_id = add_liquidity_state.position.position_id.clone();

            debug!(
                "[TEST HOOK] after_add_liquidity: Liquidity_position_id: {:?}",
                _liquidity_position_id.unwrap()
            );
            (add_liquidity_state, lp_token)
        }

        pub fn before_remove_liquidity(
            &mut self,
            state_before: BeforeRemoveLiquidityState,
        ) -> (BeforeRemoveLiquidityState,) {
            debug!("[TEST HOOK] before_remove_liquidity");
            (state_before,)
        }

        pub fn after_remove_liquidity(
            &mut self,
            state_after: AfterRemoveLiquidityState,
            mut x_output: Bucket,
            mut y_output: Bucket,
        ) -> (AfterRemoveLiquidityState, Bucket, Bucket) {
            debug!("[TEST HOOK] after_remove_liquidity");
            if self.calls_access.after_remove_liquidity_x_output {
                debug!("[TEST HOOK] after_remove_liquidity: In after_remove_liquidity_x_output");
                self.x_vault.put(x_output.take(Decimal::ZERO));
            }
            if self.calls_access.after_remove_liquidity_y_output {
                debug!("[TEST HOOK] after_remove_liquidity: In after_remove_liquidity_y_output");
                self.y_vault.put(y_output.take(Decimal::ZERO));
            }
            (state_after, x_output, y_output)
        }
    }
}

#[derive(ScryptoSbor, Clone, Debug, ManifestSbor)]
pub struct TestAccess {
    pub before_swap_input: bool,
    pub after_swap_output: bool,
    pub before_add_liquidity_x_input: bool,
    pub before_add_liquidity_y_input: bool,
    pub after_remove_liquidity_x_output: bool,
    pub after_remove_liquidity_y_output: bool,
}

impl TestAccess {
    pub fn new() -> Self {
        Self {
            before_swap_input: true,
            after_swap_output: true,
            before_add_liquidity_x_input: true,
            before_add_liquidity_y_input: true,
            after_remove_liquidity_x_output: true,
            after_remove_liquidity_y_output: true,
        }
    }
}
