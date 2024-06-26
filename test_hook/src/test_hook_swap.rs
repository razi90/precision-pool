use common::hooks::*;
use scrypto::prelude::*;

#[blueprint]
mod test_hook_input_fee_rate {
    enable_method_auth! {
        roles {
            hook_admin => updatable_by: [OWNER];
        },
        methods {
            calls => PUBLIC;
            before_swap => restrict_to: [hook_admin];
            after_swap => restrict_to: [hook_admin];
            set_input_fee_rates => PUBLIC;
            set_bucket_returned_fractions => PUBLIC;
        }
    }
    struct TestSwapHook {
        calls: Vec<HookCall>,
        x_vault: Vault,
        y_vault: Vault,
        before_swap_input_fee_rate: Option<Decimal>,
        after_swap_input_fee_rate: Option<Decimal>,
        before_swap_bucket_returned_fraction: Option<Decimal>,
        after_swap_bucket_returned_fraction: Option<Decimal>,
    }

    impl TestSwapHook {
        pub fn instantiate(
            x_address: ResourceAddress,
            y_address: ResourceAddress,
        ) -> (Global<TestSwapHook>, FungibleBucket) {
            let hook_badge = ResourceBuilder::new_fungible(OwnerRole::None)
                .divisibility(DIVISIBILITY_NONE)
                .metadata(metadata! {
                    init {
                        "name" => "Hook badge", locked;
                    }
                })
                .mint_roles(mint_roles!(
                    minter => rule!(allow_all);
                    minter_updater => rule!(deny_all);
                ))
                .mint_initial_supply(1);

            debug!("{:?}", hook_badge.resource_address());
            let hook_badge_address = hook_badge.resource_address();

            let hook_component = (Self {
                x_vault: Vault::new(x_address),
                y_vault: Vault::new(y_address),
                calls: vec![HookCall::BeforeSwap, HookCall::AfterSwap],
                before_swap_input_fee_rate: None,
                after_swap_input_fee_rate: None,
                before_swap_bucket_returned_fraction: None,
                after_swap_bucket_returned_fraction: None,
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

        pub fn before_swap(
            &mut self,
            mut before_swap_state: BeforeSwapState,
            mut input_bucket: Bucket,
        ) -> (BeforeSwapState, Bucket) {
            debug!("[TEST HOOK SIMPLE POOL] before_swap");
            if let Some(input_fee_rate) = self.before_swap_input_fee_rate {
                before_swap_state.input_fee_rate = input_fee_rate;
            }
            if let Some(returned_fraction) = self.before_swap_bucket_returned_fraction {
                input_bucket = self.deposit_partial(input_bucket, returned_fraction);
            }
            (before_swap_state, input_bucket)
        }

        pub fn after_swap(
            &mut self,
            mut after_swap_state: AfterSwapState,
            mut output_bucket: Bucket,
        ) -> (AfterSwapState, Bucket) {
            debug!("[TEST HOOK SIMPLE POOL] after_swap");
            if let Some(input_fee_rate) = self.after_swap_input_fee_rate {
                after_swap_state.input_fee_rate = input_fee_rate;
            }
            if let Some(returned_fraction) = self.after_swap_bucket_returned_fraction {
                output_bucket = self.deposit_partial(output_bucket, returned_fraction);
            }
            (after_swap_state, output_bucket)
        }

        fn deposit_partial(&mut self, mut bucket: Bucket, returned_fraction: Decimal) -> Bucket {
            let take_amount = bucket.amount() - bucket.amount() * returned_fraction;

            if bucket.resource_address() == self.x_vault.resource_address() {
                self.x_vault.put(bucket.take(take_amount));
            } else if bucket.resource_address() == self.y_vault.resource_address() {
                self.y_vault.put(bucket.take(take_amount));
            } else {
                panic!("[TestSwapHook]: Invalid bucket does not match vaults.");
            }
            bucket
        }

        pub fn set_input_fee_rates(
            &mut self,
            before_swap_input_fee_rate: Option<Decimal>,
            after_swap_input_fee_rate: Option<Decimal>,
        ) {
            self.before_swap_input_fee_rate = before_swap_input_fee_rate;
            self.after_swap_input_fee_rate = after_swap_input_fee_rate;
        }

        pub fn set_bucket_returned_fractions(
            &mut self,
            before_swap_bucket_returned_fraction: Option<Decimal>,
            after_swap_bucket_returned_fraction: Option<Decimal>,
        ) {
            self.before_swap_bucket_returned_fraction = before_swap_bucket_returned_fraction;
            self.after_swap_bucket_returned_fraction = after_swap_bucket_returned_fraction;
        }
    }
}
