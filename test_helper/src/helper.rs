use crate::constants::*;
use common::math::AttoDecimal;
use common::pools::SwapType;
use precision_pool::pool;
use pretty_assertions::assert_eq;
use radix_engine::system::system_modules::execution_trace::{
    ResourceSpecifier, ResourceSpecifier::Amount, ResourceSpecifier::Ids,
};
use registry_test_helper::RegistryTestHelper;
use scrypto::prelude::*;
use std::mem;

use precision_pool_hooks::HookCall;
use radix_transactions::builder::ManifestBuilder;
use scrypto_testenv::*;
use test_hook::test_hook::TestAccess;

static ONE_LP: [LiquidityPosition; 1] = [LiquidityPosition {
    left_bound: TICK_LEFT_BOUND,
    right_bound: TICK_RIGHT_BOUND,
    x_amount: DEC_10,
    y_amount: DEC_10,
}];

pub struct ShapePosition {
    pub left_bound: i32,
    pub right_bound: i32,
    pub x_address: ResourceAddress,
    pub x_amount: Decimal,
    pub y_address: ResourceAddress,
    pub y_amount: Decimal,
}

pub struct PoolTestHelper {
    pub registry: RegistryTestHelper,

    pub pool_address: Option<ComponentAddress>,
    pub lp_address: Option<ResourceAddress>,

    pub price_sqrt: Option<PreciseDecimal>,
}

impl PoolTestHelper {
    pub fn new() -> PoolTestHelper {
        Self::new_internal(true)
    }

    pub fn new_without_instantiate_registry() -> PoolTestHelper {
        Self::new_internal(false)
    }

    fn new_internal(instantiate_registry: bool) -> PoolTestHelper {
        let packages: HashMap<&str, &str> = vec![("registry", "registry"), ("precision_pool", ".")]
            .into_iter()
            .collect();
        Self::new_with_packages(packages, instantiate_registry)
    }

    pub fn new_with_packages(
        packages: HashMap<&str, &str>,
        instantiate_registry: bool,
    ) -> PoolTestHelper {
        let mut helper = PoolTestHelper {
            registry: RegistryTestHelper::new_with_packages(packages),

            pool_address: None,
            lp_address: None,

            price_sqrt: None,
        };

        if instantiate_registry {
            helper
                .registry
                .instantiate_default(helper.registry.admin_badge_address());
        }
        helper
    }

    pub fn instantiate_complete(
        &mut self,
        x_address: ResourceAddress,
        y_address: ResourceAddress,
        price_sqrt: PreciseDecimal,
        tick_spacing: u32,
        fee_rate_input: Decimal,
        fee_rate_flash_loan: Decimal,
        registry: ComponentAddress,
        hooks: Vec<(ComponentAddress, ResourceAddress)>,
    ) -> &mut PoolTestHelper {
        let package_address = self.registry.env.package_address("precision_pool");
        let mut manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        for hook in hooks.clone() {
            let (_, badge_address) = hook;
            manifest_builder = manifest_builder
                .withdraw_from_account(self.registry.env.account, badge_address, dec!(1))
                .take_from_worktop(
                    badge_address,
                    dec!(1),
                    self.registry.name(&badge_address.to_hex()),
                );
        }
        self.registry.env.manifest_builder =
            manifest_builder.with_name_lookup(|builder, lookup| {
                let hooks_buckets: Vec<(ComponentAddress, ManifestBucket)> = hooks
                    .iter()
                    .map(|(component_address, badge_address)| {
                        (
                            component_address.clone(),
                            lookup.bucket(self.registry.name(&badge_address.to_hex())),
                        )
                    })
                    .collect();
                builder.call_function(
                    package_address,
                    "PrecisionPool",
                    "instantiate",
                    manifest_args!(
                        x_address,
                        y_address,
                        price_sqrt,
                        tick_spacing,
                        fee_rate_input,
                        fee_rate_flash_loan,
                        registry,
                        hooks_buckets,
                        self.registry.env.dapp_definition
                    ),
                )
            });

        self.registry
            .env
            .new_instruction("instantiate", hooks.len() * 2 + 1, hooks.len() * 2);
        self
    }

    pub fn instantiate(
        &mut self,
        x_address: ResourceAddress,
        y_address: ResourceAddress,
        price_sqrt: PreciseDecimal,
        fee_rate_input: Decimal,
        fee_rate_flash_loan: Decimal,
        registry: ComponentAddress,
        hooks: Vec<(ComponentAddress, ResourceAddress)>,
    ) -> &mut PoolTestHelper {
        self.instantiate_complete(
            x_address,
            y_address,
            price_sqrt,
            1,
            fee_rate_input,
            fee_rate_flash_loan,
            registry,
            hooks,
        )
    }

    pub fn instantiate_tick_spacing(&mut self, tick_spacing: u32) -> &mut PoolTestHelper {
        self.registry
            .instantiate_default(self.registry.admin_badge_address());
        self.set_whitelist_registry();
        self.instantiate_complete(
            self.x_address(),
            self.y_address(),
            pdec!(1),
            tick_spacing,
            dec!(0),
            dec!(0.009),
            self.registry.registry_address.unwrap(),
            vec![],
        )
    }

    pub fn instantiate_with_liquidity(
        &mut self,
        x_address: ResourceAddress,
        x_amount: Decimal,
        y_address: ResourceAddress,
        y_amount: Decimal,
        price_sqrt: PreciseDecimal,
        input_fee_rate: Decimal,
        registry: ComponentAddress,
        hooks: Vec<(ComponentAddress, ResourceAddress)>,
        left_bound: i32,
        right_bound: i32,
    ) -> &mut PoolTestHelper {
        let account = self.registry.env.account;
        let package_address = self.registry.env.package_address("precision_pool");
        let mut manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        for hook in hooks.clone() {
            let (_, badge_address) = hook;
            manifest_builder = manifest_builder
                .withdraw_from_account(self.registry.env.account, badge_address, dec!(1))
                .take_from_worktop(
                    badge_address,
                    dec!(1),
                    self.registry.name(&badge_address.to_hex()),
                );
        }
        self.registry.env.manifest_builder = manifest_builder
            .withdraw_from_account(account, x_address, x_amount)
            .withdraw_from_account(account, y_address, y_amount)
            .take_from_worktop(x_address, x_amount, self.registry.name("x_bucket"))
            .take_from_worktop(y_address, y_amount, self.registry.name("y_bucket"))
            .with_name_lookup(|builder, lookup| {
                let x_bucket = lookup.bucket(self.registry.name("x_bucket"));
                let y_bucket = lookup.bucket(self.registry.name("y_bucket"));
                let hooks_buckets: Vec<(ComponentAddress, ManifestBucket)> = hooks
                    .iter()
                    .map(|(component_address, badge_address)| {
                        (
                            component_address.clone(),
                            lookup.bucket(self.registry.name(&badge_address.to_hex())),
                        )
                    })
                    .collect();
                builder.call_function(
                    package_address,
                    "PrecisionPool",
                    "instantiate_with_liquidity",
                    manifest_args!(
                        x_bucket,
                        y_bucket,
                        price_sqrt,
                        input_fee_rate,
                        dec!(0.009),
                        registry,
                        hooks_buckets,
                        self.registry.env.dapp_definition,
                        left_bound,
                        right_bound
                    ),
                )
            });

        self.registry
            .env
            .new_instruction("instantiate", 5 + hooks.len() * 2, 4 + hooks.len() * 2);
        self
    }

    pub fn add_liquidity(
        &mut self,
        left_bound: i32,
        right_bound: i32,
        x_address: ResourceAddress,
        x_amount: Decimal,
        y_address: ResourceAddress,
        y_amount: Decimal,
    ) -> &mut PoolTestHelper {
        let account = self.registry.env.account;
        let pool_address = self.pool_address.unwrap();
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        self.registry.env.manifest_builder = manifest_builder
            .withdraw_from_account(account, x_address, x_amount)
            .withdraw_from_account(account, y_address, y_amount)
            .take_from_worktop(x_address, x_amount, self.registry.name("x_bucket"))
            .take_from_worktop(y_address, y_amount, self.registry.name("y_bucket"))
            .with_name_lookup(|builder, lookup| {
                let x_bucket = lookup.bucket(self.registry.name("x_bucket"));
                let y_bucket = lookup.bucket(self.registry.name("y_bucket"));
                builder.call_method(
                    pool_address,
                    "add_liquidity",
                    manifest_args!(left_bound, right_bound, x_bucket, y_bucket),
                )
            });
        self.registry.env.new_instruction("add_liquidity", 5, 4);
        self
    }

    pub fn add_liquidity_shape(
        &mut self,
        left_bound: i32,
        right_bound: i32,
        x_address: ResourceAddress,
        x_amount: Decimal,
        y_address: ResourceAddress,
        y_amount: Decimal,
        shape_proof: Option<IndexSet<NonFungibleLocalId>>,
    ) -> &mut PoolTestHelper {
        let account = self.registry.env.account;
        let pool_address = self.pool_address.unwrap();
        let mut manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        manifest_builder = manifest_builder
            .withdraw_from_account(account, x_address, x_amount * 2)
            .withdraw_from_account(account, y_address, y_amount * 2)
            .take_from_worktop(x_address, x_amount, self.registry.name("x_bucket"))
            .take_from_worktop(y_address, y_amount, self.registry.name("y_bucket"))
            .take_from_worktop(x_address, x_amount, self.registry.name("x_bucket2"))
            .take_from_worktop(y_address, y_amount, self.registry.name("y_bucket2"));

        if let Some(proof) = shape_proof.clone() {
            manifest_builder = manifest_builder
                .create_proof_from_account_of_non_fungibles(
                    account,
                    self.lp_address.unwrap(),
                    proof,
                )
                .pop_from_auth_zone(self.registry.name("shape_proof"));
        }
        self.registry.env.manifest_builder =
            manifest_builder.with_name_lookup(|builder, lookup| {
                let x_bucket = lookup.bucket(self.registry.name("x_bucket"));
                let y_bucket = lookup.bucket(self.registry.name("y_bucket"));
                let x_bucket2 = lookup.bucket(self.registry.name("x_bucket2"));
                let y_bucket2 = lookup.bucket(self.registry.name("y_bucket2"));
                let shape_proof = match shape_proof.clone() {
                    Some(_) => Some(lookup.proof(self.registry.name("shape_proof"))),
                    None => None,
                };
                builder.call_method(
                    pool_address,
                    "add_liquidity_shape",
                    manifest_args!(
                        vec![
                            (left_bound, right_bound, x_bucket, y_bucket),
                            (left_bound, right_bound, x_bucket2, y_bucket2)
                        ],
                        shape_proof
                    ),
                )
            });
        match shape_proof {
            Some(_) => self
                .registry
                .env
                .new_instruction("add_liquidity_shape", 9, 8),
            None => self
                .registry
                .env
                .new_instruction("add_liquidity_shape", 7, 6),
        }
        self
    }

    pub fn remove_liquidity(
        &mut self,
        lp_positions: IndexSet<NonFungibleLocalId>,
    ) -> &mut PoolTestHelper {
        let account = self.registry.env.account;
        let pool_address = self.pool_address.unwrap();
        let lp_address = self.lp_address.unwrap();
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        self.registry.env.manifest_builder = manifest_builder
            .withdraw_non_fungibles_from_account(account, lp_address, lp_positions.clone())
            .take_non_fungibles_from_worktop(
                lp_address,
                lp_positions,
                self.registry.name("lp_bucket"),
            )
            .with_name_lookup(|builder, lookup| {
                let lp_bucket = lookup.bucket(self.registry.name("lp_bucket"));
                builder.call_method(pool_address, "remove_liquidity", manifest_args!(lp_bucket))
            });
        self.registry.env.new_instruction("remove_liquidity", 3, 2);
        self
    }

    pub fn removable_liquidity(
        &mut self,
        lp_position_ids: IndexSet<NonFungibleLocalId>,
    ) -> &mut PoolTestHelper {
        let pool_address = self.pool_address.unwrap();
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        let lp_position_ids: Vec<NonFungibleLocalId> = lp_position_ids.into_iter().collect();
        self.registry.env.manifest_builder = manifest_builder.call_method(
            pool_address,
            "removable_liquidity",
            manifest_args!(lp_position_ids),
        );
        self.registry
            .env
            .new_instruction("removable_liquidity", 1, 0);
        self
    }

    pub fn swap(
        &mut self,
        input_address: ResourceAddress,
        input_amount: Decimal,
    ) -> &mut PoolTestHelper {
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        self.registry.env.manifest_builder = manifest_builder
            .withdraw_from_account(self.registry.env.account, input_address, input_amount)
            .take_from_worktop(
                input_address,
                input_amount,
                self.registry.name("input_bucket"),
            )
            .with_name_lookup(|builder, lookup| {
                let input_bucket = lookup.bucket(self.registry.name("input_bucket"));
                builder.call_method(
                    self.pool_address.unwrap(),
                    "swap",
                    manifest_args!(input_bucket),
                )
            });
        self.registry.env.new_instruction("swap", 3, 2);
        self
    }

    pub fn claim_fees(
        &mut self,
        lp_positions: IndexSet<NonFungibleLocalId>,
    ) -> &mut PoolTestHelper {
        let account = self.registry.env.account;
        let pool_address = self.pool_address.unwrap();
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        self.registry.env.manifest_builder = manifest_builder
            .create_proof_from_account_of_non_fungibles(
                account,
                self.lp_address.unwrap(),
                lp_positions,
            )
            .pop_from_auth_zone(self.registry.name("lp_proof"))
            .with_name_lookup(|builder, lookup| {
                let lp_proofs = lookup.proof(self.registry.name("lp_proof"));
                builder.call_method(pool_address, "claim_fees", manifest_args!(lp_proofs))
            });
        self.registry.env.new_instruction("claim_fees", 3, 2);
        self
    }

    pub fn claimable_fees(
        &mut self,
        lp_position_ids: IndexSet<NonFungibleLocalId>,
    ) -> &mut PoolTestHelper {
        let pool_address = self.pool_address.unwrap();
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        let lp_position_ids: Vec<NonFungibleLocalId> = lp_position_ids.into_iter().collect();
        self.registry.env.manifest_builder = manifest_builder.call_method(
            pool_address,
            "claimable_fees",
            manifest_args!(lp_position_ids),
        );
        self.registry.env.new_instruction("claimable_fees", 1, 0);
        self
    }

    pub fn total_fees(
        &mut self,
        lp_position_ids: IndexSet<NonFungibleLocalId>,
    ) -> &mut PoolTestHelper {
        let pool_address = self.pool_address.unwrap();
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        let lp_position_ids: Vec<NonFungibleLocalId> = lp_position_ids.into_iter().collect();
        self.registry.env.manifest_builder = manifest_builder.call_method(
            pool_address,
            "total_fees",
            manifest_args!(lp_position_ids),
        );
        self.registry.env.new_instruction("total_fees", 1, 0);
        self
    }

    pub fn seconds_in_position(&mut self, nft_id: NonFungibleLocalId) -> &mut PoolTestHelper {
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        self.registry.env.manifest_builder = manifest_builder.call_method(
            self.pool_address.unwrap(),
            "seconds_in_position",
            manifest_args!(nft_id),
        );
        self.registry
            .env
            .new_instruction("seconds_in_position", 1, 0);
        self
    }

    pub fn seconds_in_position_batch_success(
        &mut self,
        seconds_inside_tests: &Vec<SecondsInsideTest>,
    ) -> &mut PoolTestHelper {
        for seconds_inside_test in seconds_inside_tests {
            self.seconds_in_position(seconds_inside_test.nft_id.clone());
        }

        let outputs: Vec<u64> = self
            .registry
            .execute_expect_success(false)
            .outputs("seconds_in_position");

        let mut seconds_inside_tests_output = seconds_inside_tests.clone();
        for i in 0..seconds_inside_tests.len() {
            seconds_inside_tests_output[i].seconds_in_position = outputs[i];
        }

        assert_eq!(seconds_inside_tests_output, *seconds_inside_tests);

        self
    }

    pub fn flash_loan(
        &mut self,
        loan_address: ResourceAddress,
        loan_amount: Decimal,
    ) -> &mut PoolTestHelper {
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        self.registry.env.manifest_builder = manifest_builder.call_method(
            self.pool_address.unwrap(),
            "flash_loan",
            manifest_args!(loan_address, loan_amount),
        );
        self.registry.env.new_instruction("flash_loan", 1, 0);
        self
    }

    pub fn repay_loan(
        &mut self,
        repay_address: ResourceAddress,
        repay_amount: Decimal,
        repay_fee_amount: Decimal,
        flash_loan_address: ResourceAddress,
        transient_amount: Decimal,
    ) -> &mut PoolTestHelper {
        let manifest_builder = mem::take(&mut self.registry.env.manifest_builder);
        let account_component = self.registry.env.account;
        self.registry.env.manifest_builder = manifest_builder
            .withdraw_from_account(account_component, repay_address, repay_fee_amount)
            .take_from_worktop(
                repay_address,
                repay_amount + repay_fee_amount,
                self.registry.name("repay_bucket"),
            )
            .take_from_worktop(
                flash_loan_address,
                transient_amount,
                self.registry.name("transient_bucket"),
            )
            .with_name_lookup(|builder, lookup| {
                let repay_bucket = lookup.bucket(self.registry.name("repay_bucket"));
                let transient_bucket = lookup.bucket(self.registry.name("transient_bucket"));
                builder.call_method(
                    self.pool_address.unwrap(),
                    "repay_loan",
                    manifest_args!(repay_bucket, transient_bucket),
                )
            });
        self.registry.env.new_instruction("repay_loan", 4, 3);
        self
    }

    pub fn repay_loan_success(
        &mut self,
        repay_address: ResourceAddress,
        loan_amount: Decimal,
        repay_amount: Decimal,
        repay_fee_amount: Decimal,
        remainder_expected: Decimal,
    ) -> &mut PoolTestHelper {
        self.instantiate_default_with_flash_loan_fee(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            dec!(0.009),
            false,
        );
        let receipt = self
            .add_liquidity_default_batch(&ONE_LP)
            .flash_loan_address()
            .registry
            .execute_expect_success(false);
        let transient_address: ResourceAddress = receipt.outputs("flash_loan_address")[0];

        self.flash_loan(self.x_address(), loan_amount);
        let receipt = self
            .repay_loan(
                repay_address,
                repay_amount,
                repay_fee_amount,
                transient_address,
                dec!(1),
            )
            .registry
            .execute_expect_success(false);
        let output_buckets = receipt.output_buckets("repay_loan");
        assert_eq!(
            output_buckets,
            vec![vec![Amount(repay_address, remainder_expected)]]
        );

        self
    }

    pub fn repay_loan_failure(
        &mut self,
        repay_address: ResourceAddress,
        repay_amount: Decimal,
        repay_fee_amount: Decimal,
    ) -> &mut PoolTestHelper {
        self.instantiate_default_with_flash_loan_fee(
            *PRICE_BETWEEN_MIDDLE_BOUNDS_SQRT,
            dec!(0.009),
            false,
        );
        let receipt = self
            .add_liquidity_default_batch(&ONE_LP)
            .flash_loan_address()
            .registry
            .execute_expect_success(false);
        let transient_address: ResourceAddress = receipt.outputs("flash_loan_address")[0];

        self.flash_loan(self.x_address(), dec!(1));
        self.repay_loan(
            repay_address,
            repay_amount,
            repay_fee_amount,
            transient_address,
            dec!(1),
        )
        .registry
        .execute_expect_failure(false);

        self
    }

    pub fn add_liquidity_default(
        &mut self,
        left_bound: i32,
        right_bound: i32,
        x_amount: Decimal,
        y_amount: Decimal,
    ) -> &mut PoolTestHelper {
        self.add_liquidity(
            left_bound,
            right_bound,
            self.x_address(),
            x_amount,
            self.y_address(),
            y_amount,
        );
        self
    }

    pub fn add_liquidity_default_batch(
        &mut self,
        positions: &[LiquidityPosition],
    ) -> &mut PoolTestHelper {
        for position in positions {
            self.add_liquidity_default(
                position.left_bound,
                position.right_bound,
                position.x_amount,
                position.y_amount,
            );
        }
        self
    }

    pub fn price_sqrt(&mut self) -> &mut PoolTestHelper {
        self.getter("price_sqrt")
    }

    pub fn total_liquidity(&mut self) -> &mut PoolTestHelper {
        self.getter("total_liquidity")
    }

    pub fn input_fee_rate(&mut self) -> &mut PoolTestHelper {
        self.getter("input_fee_rate")
    }

    pub fn fee_protocol_rate(&mut self) -> &mut PoolTestHelper {
        self.getter("fee_protocol_rate")
    }

    pub fn flash_loan_fee_rate(&mut self) -> &mut PoolTestHelper {
        self.getter("flash_loan_fee_rate")
    }

    pub fn lp_address(&mut self) -> &mut PoolTestHelper {
        self.getter("lp_address")
    }

    pub fn getter(&mut self, name: &str) -> &mut PoolTestHelper {
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        self.registry.env.manifest_builder =
            manifest_builder.call_method(self.pool_address.unwrap(), name, manifest_args!());
        self.registry.env.new_instruction(name, 1, 0);
        self
    }

    pub fn execute_after_instantiate(
        &mut self,
        price_sqrt: PreciseDecimal,
        a_address: ResourceAddress,
        b_address: ResourceAddress,
    ) -> &mut PoolTestHelper {
        let pool_address = self.pool_address.unwrap();
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        self.registry.env.manifest_builder = manifest_builder.call_method(
            pool_address,
            "execute_after_instantiate",
            manifest_args!(pool_address, price_sqrt, a_address, b_address),
        );
        self.registry
            .env
            .new_instruction("execute_after_instantiate", 1, 0);
        self
    }

    pub fn flash_loan_address(&mut self) -> &mut PoolTestHelper {
        let manifest_builder = mem::replace(
            &mut self.registry.env.manifest_builder,
            ManifestBuilder::new(),
        );
        self.registry.env.manifest_builder = manifest_builder.call_method(
            self.pool_address.unwrap(),
            "flash_loan_address",
            manifest_args!(),
        );
        self.registry
            .env
            .new_instruction("flash_loan_address", 1, 0);
        self
    }

    pub fn instantiate_default(
        &mut self,
        price_sqrt: PreciseDecimal,
        verbose: bool,
    ) -> &mut PoolTestHelper {
        self.instantiate_default_with_fees_and_hooks(price_sqrt, dec!(0), dec!(0), vec![], verbose)
    }

    pub fn instantiate_default_with_input_fee(
        &mut self,
        price_sqrt: PreciseDecimal,
        input_fee_rate: Decimal,
        verbose: bool,
    ) -> &mut PoolTestHelper {
        self.instantiate_default_with_fees_and_hooks(
            price_sqrt,
            input_fee_rate,
            dec!(0),
            vec![],
            verbose,
        )
    }

    pub fn instantiate_default_with_flash_loan_fee(
        &mut self,
        price_sqrt: PreciseDecimal,
        flash_loan_fee_rate: Decimal,
        verbose: bool,
    ) -> &mut PoolTestHelper {
        self.instantiate_default_with_fees_and_hooks(
            price_sqrt,
            dec!(0),
            flash_loan_fee_rate,
            vec![],
            verbose,
        )
    }

    pub fn instantiate_default_with_hooks(
        &mut self,
        price_sqrt: PreciseDecimal,
        hooks: Vec<(ComponentAddress, ResourceAddress)>,
        verbose: bool,
    ) -> &mut PoolTestHelper {
        self.instantiate_default_with_fees_and_hooks(price_sqrt, dec!(0), dec!(0), hooks, verbose)
    }

    pub fn instantiate_default_with_fees_and_hooks(
        &mut self,
        price_sqrt: PreciseDecimal,
        input_fee_rate: Decimal,
        flash_loan_fee_rate: Decimal,
        hooks: Vec<(ComponentAddress, ResourceAddress)>,
        verbose: bool,
    ) -> &mut PoolTestHelper {
        self.set_whitelist_registry();
        self.instantiate(
            self.x_address(),
            self.y_address(),
            price_sqrt,
            input_fee_rate,
            flash_loan_fee_rate,
            self.registry.registry_address.unwrap(),
            hooks,
        );
        let receipt = self.registry.execute_expect_success(verbose);
        let (pool_address, lp_address): (ComponentAddress, ResourceAddress) =
            receipt.outputs("instantiate")[0];
        self.pool_address = Some(pool_address);
        self.lp_address = Some(lp_address);
        self.price_sqrt = Some(price_sqrt);
        self
    }

    pub fn new_default_with_positions(
        price_sqrt: PreciseDecimal,
        positions: &[LiquidityPosition],
    ) -> PoolTestHelper {
        let mut helper = PoolTestHelper::new();
        helper.instantiate_default(price_sqrt, false);
        helper.add_liquidity_default_batch(positions);
        helper
    }

    pub fn swap_x_default(&mut self, x_amount: Decimal) -> &mut PoolTestHelper {
        self.swap(self.x_address(), x_amount);
        self
    }

    pub fn swap_y_default(&mut self, y_amount: Decimal) -> &mut PoolTestHelper {
        self.swap(self.y_address(), y_amount);
        self
    }

    pub fn tick_spacing(&mut self) -> &mut PoolTestHelper {
        let manifest_builder = mem::take(&mut self.registry.env.manifest_builder);
        self.registry.env.manifest_builder = manifest_builder.call_method(
            self.pool_address.unwrap(),
            "tick_spacing",
            manifest_args!(),
        );
        self.registry.env.new_instruction("tick_spacing", 1, 0);
        self
    }

    pub fn swap_by_type(&mut self, swap_type: SwapType, amount: Decimal) -> &mut PoolTestHelper {
        let _ = match swap_type {
            SwapType::BuyX => self.swap(self.y_address(), amount),
            SwapType::SellX => self.swap(self.x_address(), amount),
        };

        self
    }

    pub fn sync_registry(&mut self) -> &mut PoolTestHelper {
        let manifest_builder = mem::take(&mut self.registry.env.manifest_builder);
        self.registry.env.manifest_builder = manifest_builder.call_method(
            self.pool_address.unwrap(),
            "sync_registry",
            manifest_args!(),
        );
        self.registry.env.new_instruction("sync_registry", 1, 0);
        self
    }

    pub fn next_sync_time(&mut self) -> &mut PoolTestHelper {
        let manifest_builder = mem::take(&mut self.registry.env.manifest_builder);
        self.registry.env.manifest_builder = manifest_builder.call_method(
            self.pool_address.unwrap(),
            "next_sync_time",
            manifest_args!(),
        );
        self.registry.env.new_instruction("next_sync_time", 1, 0);
        self
    }

    pub fn active_tick(&mut self) -> &mut PoolTestHelper {
        let manifest_builder = mem::take(&mut self.registry.env.manifest_builder);
        self.registry.env.manifest_builder = manifest_builder.call_method(
            self.pool_address.unwrap(),
            "active_tick",
            manifest_args!(),
        );
        self.registry.env.new_instruction("active_tick", 1, 0);
        self
    }

    pub fn a_address(&self) -> ResourceAddress {
        self.registry.env.a_address
    }

    pub fn b_address(&self) -> ResourceAddress {
        self.registry.env.b_address
    }

    pub fn x_address(&self) -> ResourceAddress {
        self.registry.env.x_address
    }

    pub fn y_address(&self) -> ResourceAddress {
        self.registry.env.y_address
    }

    pub fn v_address(&self) -> ResourceAddress {
        self.registry.env.v_address
    }

    pub fn u_address(&self) -> ResourceAddress {
        self.registry.env.u_address
    }

    pub fn j_nft_address(&self) -> ResourceAddress {
        self.registry.env.j_nft_address
    }

    pub fn k_nft_address(&self) -> ResourceAddress {
        self.registry.env.k_nft_address
    }

    pub fn admin_badge_address(&self) -> ResourceAddress {
        self.registry.env.admin_badge_address
    }

    pub fn add_liquidity_failure(
        &mut self,
        left_bound: i32,
        right_bound: i32,
        x_input: Decimal,
        y_input: Decimal,
    ) {
        self.add_liquidity_default(left_bound, right_bound, x_input, y_input)
            .registry
            .execute_expect_failure(false);
    }

    pub fn add_liquidity_success(
        &mut self,
        left_bound: i32,
        right_bound: i32,
        x_input: Decimal,
        y_input: Decimal,
        x_output: Decimal,
        y_output: Decimal,
    ) -> Vec<Vec<ResourceSpecifier>> {
        let receipt = self
            .add_liquidity_default(left_bound, right_bound, x_input, y_input)
            .registry
            .execute_expect_success(false);
        let output_buckets = receipt.output_buckets("add_liquidity");

        assert_eq!(
            output_buckets,
            vec![vec![
                Ids(self.lp_address.unwrap(), nft_ids!(1)),
                Amount(self.x_address(), x_output),
                Amount(self.y_address(), y_output)
            ]],
            "\nprice_sqrt={:?}, Y LB: {:?}, RB: {:?}, XA: {:?}, YA: {:?}",
            self.price_sqrt,
            left_bound,
            right_bound,
            x_input,
            y_input
        );

        output_buckets
    }

    pub fn swap_success(
        &mut self,
        swap_type: SwapType,
        input_amount: Decimal,
        output_amount: Decimal,
        remainder_amount: Decimal,
    ) {
        let input_address = self.input_address(swap_type);
        let receipt = self
            .swap(input_address, input_amount)
            .registry
            .execute_expect_success(false);
        let output_buckets = receipt.output_buckets("swap");

        let (output_address, remainder_address) = if self.x_address() == input_address {
            (self.y_address(), self.x_address())
        } else {
            (self.x_address(), self.y_address())
        };

        assert_eq!(
            output_buckets,
            vec![vec![
                Amount(output_address, output_amount),
                Amount(remainder_address, remainder_amount)
            ]],
            "\nInput: Address {:?} Amount {:?}, Output: Address {:?} Amount {:?}",
            input_address,
            input_amount,
            output_address,
            output_amount
        );
    }

    pub fn swap_failure(&mut self, swap_type: SwapType, input_amount: Decimal) {
        let input_address = self.input_address(swap_type);
        self.swap(input_address, input_amount)
            .registry
            .execute_expect_failure(false);
    }

    pub fn remove_liquidity_success(
        &mut self,
        lp_positions: IndexSet<NonFungibleLocalId>,
        x_output_expected: Decimal,
        y_output_expected: Decimal,
    ) {
        let receipt = self
            .remove_liquidity(lp_positions)
            .registry
            .execute_expect_success(false);
        let output_buckets = receipt.output_buckets("remove_liquidity");

        assert_eq!(
            output_buckets,
            vec![vec![
                Amount(self.x_address(), x_output_expected),
                Amount(self.y_address(), y_output_expected)
            ]],
            "\nX Amount = {:?}, Y Amount {:?}",
            x_output_expected,
            y_output_expected
        );
    }

    pub fn removable_liquidity_success(
        &mut self,
        lp_positions: IndexSet<NonFungibleLocalId>,
        x_output_expected: Decimal,
        y_output_expected: Decimal,
        minimum_removable_fraction: Decimal,
    ) {
        let receipt = self
            .removable_liquidity(lp_positions)
            .registry
            .execute_expect_success(false);
        let output_amounts: Vec<(IndexMap<ResourceAddress, Decimal>, Decimal)> =
            receipt.outputs("removable_liquidity");

        assert_eq!(
            output_amounts,
            vec![(
                IndexMap::from([
                    (self.x_address(), x_output_expected),
                    (self.y_address(), y_output_expected),
                ]),
                minimum_removable_fraction
            )],
            "\nX Amount = {:?}, Y Amount {:?}",
            x_output_expected,
            y_output_expected
        );
    }

    pub fn claim_fees_success(
        &mut self,
        lp_positions: IndexSet<NonFungibleLocalId>,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
    ) {
        let receipt = self
            .claim_fees(lp_positions)
            .registry
            .execute_expect_success(false);
        let output_buckets = receipt.output_buckets("claim_fees");

        assert_eq!(
            output_buckets,
            vec![vec![
                Amount(self.x_address(), x_fee_expected),
                Amount(self.y_address(), y_fee_expected)
            ]],
            "\nX Amount = {:?}, Y Amount {:?}",
            x_fee_expected,
            y_fee_expected
        );
    }

    pub fn claimable_fees_success(
        &mut self,
        lp_positions: IndexSet<NonFungibleLocalId>,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
    ) {
        let receipt = self
            .claimable_fees(lp_positions)
            .registry
            .execute_expect_success(false);
        let output_amounts: Vec<IndexMap<ResourceAddress, Decimal>> =
            receipt.outputs("claimable_fees");

        assert_eq!(
            output_amounts,
            vec![IndexMap::from([
                (self.x_address(), x_fee_expected),
                (self.y_address(), y_fee_expected),
            ])],
            "\nX Amount = {:?}, Y Amount {:?}",
            x_fee_expected,
            y_fee_expected
        );
    }

    pub fn total_fees_success(
        &mut self,
        lp_position_ids: IndexSet<NonFungibleLocalId>,
        x_fee_expected: Decimal,
        y_fee_expected: Decimal,
    ) {
        let receipt = self
            .total_fees(lp_position_ids)
            .registry
            .execute_expect_success(false);
        let output_amounts: Vec<IndexMap<ResourceAddress, Decimal>> = receipt.outputs("total_fees");

        assert_eq!(
            output_amounts,
            vec![IndexMap::from([
                (self.x_address(), x_fee_expected),
                (self.y_address(), y_fee_expected),
            ])],
            "\nX Amount = {:?}, Y Amount {:?}",
            x_fee_expected,
            y_fee_expected
        );
    }

    pub fn input_address(&self, swap_type: SwapType) -> ResourceAddress {
        match swap_type {
            SwapType::SellX => self.x_address(),
            SwapType::BuyX => self.y_address(),
        }
    }

    pub fn jump_to_timestamp_seconds(&mut self, seconds: u64) {
        let current_time = self
            .registry
            .env
            .test_runner
            .get_current_time(TimePrecision::Second)
            .seconds_since_unix_epoch as u64;
        if current_time == seconds {
            return;
        }

        let current_round = self
            .registry
            .env
            .test_runner
            .get_consensus_manager_state()
            .round
            .number();
        self.registry
            .env()
            .test_runner
            .advance_to_round_at_timestamp(Round::of(current_round + 1), (seconds * 1000) as i64);
    }

    pub fn advance_timestamp_by_seconds(&mut self, seconds: u64) {
        let current_time = self
            .registry
            .env()
            .test_runner
            .get_current_time(TimePrecision::Second)
            .seconds_since_unix_epoch as u64;
        self.jump_to_timestamp_seconds(current_time + seconds)
    }

    pub fn jump_to_timestamp_minutes(&mut self, minutes: u64) {
        self.jump_to_timestamp_seconds(minutes * 60);
    }

    pub fn advance_timestamp_by_minutes(&mut self, minutes: u64) {
        self.advance_timestamp_by_seconds(minutes * 60);
    }

    pub fn last_observation_index(&mut self) -> &mut PoolTestHelper {
        let manifest_builder = mem::take(&mut self.registry.env.manifest_builder);
        self.registry.env.manifest_builder = manifest_builder.call_method(
            self.pool_address.unwrap(),
            "last_observation_index",
            manifest_args!(),
        );
        self.registry
            .env
            .new_instruction("last_observation_index", 1, 0);
        self
    }

    pub fn oldest_observation_at(&mut self) -> &mut PoolTestHelper {
        let manifest_builder = mem::take(&mut self.registry.env.manifest_builder);
        self.registry.env.manifest_builder = manifest_builder.call_method(
            self.pool_address.unwrap(),
            "oldest_observation_at",
            manifest_args!(),
        );
        self.registry
            .env
            .new_instruction("oldest_observation_at", 1, 0);
        self
    }

    pub fn set_whitelist_registry(&mut self) -> &mut PoolTestHelper {
        self.set_whitelist_packages("registry_packages", vec!["registry"])
    }

    pub fn set_whitelist_registry_value(
        &mut self,
        value: impl ToMetadataEntry,
    ) -> &mut PoolTestHelper {
        self.set_metadata("registry_packages", value)
    }

    pub fn set_whitelist_hook(&mut self, package_name: &str) -> &mut PoolTestHelper {
        self.set_whitelist_packages("hook_packages", vec![package_name])
    }

    pub fn set_whitelist_packages(
        &mut self,
        metadata_key: &str,
        package_names: Vec<&str>,
    ) -> &mut PoolTestHelper {
        let global_package_addresses: Vec<GlobalAddress> = package_names
            .iter()
            .map(|package_name| self.registry.env.package_address(package_name).into())
            .collect();
        self.set_metadata(metadata_key, global_package_addresses)
    }

    pub fn set_metadata(
        &mut self,
        key: impl Into<String>,
        value: impl ToMetadataEntry,
    ) -> &mut PoolTestHelper {
        let precision_pool_package_address: GlobalAddress =
            self.registry.env.package_address("precision_pool").into();
        let manifest_builder = mem::take(&mut self.registry.env.manifest_builder);
        self.registry.env.manifest_builder = manifest_builder
            .create_proof_from_account_of_amount(
                self.registry.env().account,
                self.admin_badge_address(),
                dec!(1),
            )
            .set_metadata(precision_pool_package_address, key, value);
        self.registry.env.new_instruction("set_metadata", 2, 1);
        self
    }
}

pub fn add_liquidity_expect_failure(
    price_sqrt: PreciseDecimal,
    left_bound: i32,
    right_bound: i32,
    x_input: Decimal,
    y_input: Decimal,
) {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(price_sqrt, false);
    helper.add_liquidity_failure(left_bound, right_bound, x_input, y_input);
}

pub fn add_liquidity_expect_success(
    price_sqrt: PreciseDecimal,
    left_bound: i32,
    right_bound: i32,
    x_input: Decimal,
    y_input: Decimal,
    x_output_expected: Decimal,
    y_output_expected: Decimal,
) {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(price_sqrt, false);
    helper.add_liquidity_success(
        left_bound,
        right_bound,
        x_input,
        y_input,
        x_output_expected,
        y_output_expected,
    );
}

pub fn add_liquidity_bounds_expect_failure(
    price_sqrt: PreciseDecimal,
    left_bound: i32,
    right_bound: i32,
) {
    add_liquidity_expect_failure(price_sqrt, left_bound, right_bound, dec!(1), dec!(1));
}

pub fn add_liquidity_tick_spacing_assert_ticks(
    tick_spacing: u32,
    left_bound: i32,
    right_bound: i32,
    left_bound_expected: i32,
    right_bound_expected: i32,
    update_existing_tick: bool,
    expect_success: bool,
) {
    let mut helper = PoolTestHelper::new();
    let receipt = helper
        .instantiate_tick_spacing(tick_spacing)
        .registry
        .execute_expect_success(false);

    let (pool_address, lp_address): (ComponentAddress, ResourceAddress) =
        receipt.outputs("instantiate")[0];
    helper.pool_address = Some(pool_address);
    helper.lp_address = Some(lp_address);

    if update_existing_tick {
        helper
            .add_liquidity_default(
                left_bound,
                right_bound,
                Decimal::ATTO * 10,
                Decimal::ATTO * 10,
            )
            .registry
            .execute_expect_success(false);
    }

    helper.add_liquidity_default(left_bound, right_bound, dec!(1), dec!(1));

    if expect_success == true {
        helper.registry.execute_expect_success(false);
    } else {
        helper.registry.execute_expect_failure(false);
        return;
    }

    let metadata: pool::LiquidityPosition = helper.registry.env.test_runner.get_non_fungible_data(
        helper.lp_address.unwrap(),
        if update_existing_tick {
            nft_id!(2)
        } else {
            nft_id!(1)
        },
    );

    assert_eq!(
        (metadata.left_bound, metadata.right_bound),
        (left_bound_expected, right_bound_expected)
    );
}

pub fn add_liquidity_tick_spacing_assert_amount(
    tick_spacing: u32,
    left_bound: i32,
    right_bound: i32,
    x_amount: Decimal,
    y_amount: Decimal,
    update_existing_tick: bool,
    expect_success: bool,
) {
    let mut helper = PoolTestHelper::new();
    let receipt = helper
        .instantiate_tick_spacing(tick_spacing)
        .registry
        .execute_expect_success(false);

    let (pool_address, lp_address): (ComponentAddress, ResourceAddress) =
        receipt.outputs("instantiate")[0];
    helper.pool_address = Some(pool_address);
    helper.lp_address = Some(lp_address);

    if update_existing_tick {
        helper
            .add_liquidity_default(
                left_bound,
                right_bound,
                Decimal::ATTO * 10,
                Decimal::ATTO * 10,
            )
            .registry
            .execute_expect_success(false);
    }

    helper.add_liquidity_default(left_bound, right_bound, x_amount, y_amount);

    if expect_success == true {
        helper.registry.execute_expect_success(false);
    } else {
        helper.registry.execute_expect_failure(false);
        return;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LiquidityPosition {
    pub left_bound: i32,
    pub right_bound: i32,
    pub x_amount: Decimal,
    pub y_amount: Decimal,
}

#[derive(Clone, Copy, Debug)]
pub struct Trade {
    pub type_: SwapType,
    pub input_amount: Decimal,
    pub output_expected: Decimal,
    pub remainder_expected: Decimal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SecondsInsideTest {
    pub nft_id: NonFungibleLocalId,
    pub seconds_in_position: u64,
}

pub fn swap_success(price_sqrt: PreciseDecimal, positions: &[LiquidityPosition], trades: &[Trade]) {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(price_sqrt, false);
    for position in positions {
        helper.add_liquidity_default(
            position.left_bound,
            position.right_bound,
            position.x_amount,
            position.y_amount,
        );
    }
    for trade in trades {
        helper.swap_success(
            trade.type_,
            trade.input_amount,
            trade.output_expected,
            trade.remainder_expected,
        );
    }
}

pub fn swap_buy_success(
    price_sqrt: PreciseDecimal,
    positions: &[LiquidityPosition],
    y_input: Decimal,
    x_output_expected: Decimal,
    y_remainder_expected: Decimal,
) {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(price_sqrt, false);
    for position in positions {
        helper.add_liquidity_default(
            position.left_bound,
            position.right_bound,
            position.x_amount,
            position.y_amount,
        );
    }
    helper.registry.execute_expect_success(false);
    helper.swap_success(
        SwapType::BuyX,
        y_input,
        x_output_expected,
        y_remainder_expected,
    );
}

pub fn swap_buy_failure(
    price_sqrt: PreciseDecimal,
    positions: &[LiquidityPosition],
    y_input: Decimal,
) {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(price_sqrt, false);
    for position in positions {
        helper.add_liquidity_default(
            position.left_bound,
            position.right_bound,
            position.x_amount,
            position.y_amount,
        );
    }
    helper.swap_failure(SwapType::BuyX, y_input);
}

pub fn swap_sell_success(
    price_sqrt: PreciseDecimal,
    positions: &[LiquidityPosition],
    x_input: Decimal,
    y_output_expected: Decimal,
    x_remainder_expected: Decimal,
) {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(price_sqrt, false);
    helper.add_liquidity_default_batch(positions);
    helper.swap_success(
        SwapType::SellX,
        x_input,
        y_output_expected,
        x_remainder_expected,
    );
}

pub fn swap_sell_failure(
    price_sqrt: PreciseDecimal,
    positions: &[LiquidityPosition],
    x_input: Decimal,
) {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(price_sqrt, false);
    helper.add_liquidity_default_batch(positions);
    helper.swap_failure(SwapType::SellX, x_input);
}

/// Helper for the default remove liquidity scenario: ADD, ADD, REMOVE, SWAP, REMOVE
/// Asserting on the output of the swap and final remove liquidity
pub fn remove_liquidity_default_scenario(
    price_sqrt: PreciseDecimal,
    positions: &[LiquidityPosition],
    remove_first_first: bool,
    swap_type: SwapType,
    swap_input: Decimal,
    swap_output_expected: Decimal,
    swap_remainder_expected: Decimal,
    x_expected: Decimal,
    y_expected: Decimal,
) {
    let mut helper = PoolTestHelper::new();
    helper.instantiate_default(price_sqrt, false);
    helper
        .add_liquidity_default_batch(positions)
        .registry
        .execute_expect_success(false);

    helper.remove_liquidity(nft_ids!(if remove_first_first { 1 } else { 2 }));

    let swap_input_address = match swap_type {
        SwapType::BuyX => helper.y_address(),
        SwapType::SellX => helper.x_address(),
    };
    helper.swap(swap_input_address, swap_input);
    helper.remove_liquidity(nft_ids!(if remove_first_first { 2 } else { 1 }));

    let receipt = helper.registry.execute_expect_success(false);

    let (swap_output_address, swap_remainder_address) = if helper.x_address() == swap_input_address
    {
        (helper.y_address(), helper.x_address())
    } else {
        (helper.x_address(), helper.y_address())
    };

    assert_eq!(
        receipt.output_buckets("swap"),
        vec![vec![
            Amount(swap_output_address, swap_output_expected),
            Amount(swap_remainder_address, swap_remainder_expected)
        ]]
    );

    let output_buckets = receipt.output_buckets("remove_liquidity");

    assert_eq!(
        output_buckets[1],
        vec![
            Amount(helper.x_address(), x_expected),
            Amount(helper.y_address(), y_expected)
        ]
    );
}

pub fn remove_liquidity_default_scenario_buy(
    price_sqrt: PreciseDecimal,
    positions: &[LiquidityPosition],
    remove_first_first: bool,
    swap_input: Decimal,
    swap_output_expected: Decimal,
    swap_remainder_expected: Decimal,
    x_expected: Decimal,
    y_expected: Decimal,
) {
    remove_liquidity_default_scenario(
        price_sqrt,
        positions,
        remove_first_first,
        SwapType::BuyX,
        swap_input,
        swap_output_expected,
        swap_remainder_expected,
        x_expected,
        y_expected,
    )
}

pub fn remove_liquidity_default_scenario_sell(
    price_sqrt: PreciseDecimal,
    positions: &[LiquidityPosition],
    remove_first_first: bool,
    swap_input: Decimal,
    swap_output_expected: Decimal,
    swap_remainder_expected: Decimal,
    x_expected: Decimal,
    y_expected: Decimal,
) {
    remove_liquidity_default_scenario(
        price_sqrt,
        positions,
        remove_first_first,
        SwapType::SellX,
        swap_input,
        swap_output_expected,
        swap_remainder_expected,
        x_expected,
        y_expected,
    )
}

pub fn swap_with_hook_action_test(
    method_name: &str,
    before_swap_fee_rate: Option<Decimal>,
    after_swap_fee_rate: Option<Decimal>,
    expect_success: bool,
) {
    let packages: HashMap<&str, &str> = vec![
        ("registry", "registry"),
        ("precision_pool", "."),
        ("test_hook", "test_hook"),
    ]
    .into_iter()
    .collect();
    let mut helper = PoolTestHelper::new_with_packages(packages, true);

    helper.set_whitelist_hook("test_hook");

    let package_address = helper.registry.env.package_address("test_hook");
    let manifest_builder = mem::replace(
        &mut helper.registry.env.manifest_builder,
        ManifestBuilder::new(),
    );
    helper.registry.env.manifest_builder = manifest_builder.call_function(
        package_address,
        "TestSwapHook",
        "instantiate",
        manifest_args!(helper.x_address(), helper.y_address()),
    );
    helper
        .registry
        .env
        .new_instruction("instantiate_test_hook", 1, 0);

    let receipt = helper.registry.execute_expect_success(false);

    let new_resource_ads = receipt
        .execution_receipt
        .expect_commit_success()
        .new_resource_addresses();

    let outputs: Vec<(ComponentAddress, Bucket)> = receipt.outputs("instantiate_test_hook");

    let hook_address = outputs[0].0;
    let hook_badge_address = new_resource_ads[0];

    let hook_infos = vec![(hook_address, hook_badge_address)];

    helper.instantiate_default_with_hooks(pdec!(1), hook_infos, false);
    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    let manifest_builder = mem::replace(
        &mut helper.registry.env.manifest_builder,
        ManifestBuilder::new(),
    );
    helper.registry.env.manifest_builder = manifest_builder.call_method(
        hook_address,
        method_name,
        manifest_args!(before_swap_fee_rate, after_swap_fee_rate),
    );
    helper.registry.execute_expect_success(false);

    helper.swap(helper.input_address(SwapType::BuyX), dec!(1));

    if expect_success {
        helper.registry.execute_expect_success(false);
    } else {
        helper.registry.execute_expect_failure(false);
    }
}

pub fn removable_liquidity_with_remove_hook(
    lp_positions: IndexSet<NonFungibleLocalId>,
    x_output_expected: Decimal,
    y_output_expected: Decimal,
    minimum_removable_fraction_expected: Decimal,
) {
    let packages: HashMap<&str, &str> = vec![
        ("registry", "registry"),
        ("precision_pool", "."),
        ("test_hook", "test_hook"),
    ]
    .into_iter()
    .collect();
    let mut helper = PoolTestHelper::new_with_packages(packages, true);

    helper.set_whitelist_registry();
    helper.set_whitelist_hook("test_hook");
    helper.registry.execute_expect_success(false);

    let package_address = helper.registry.env.package_address("test_hook");
    let manifest_builder = mem::replace(
        &mut helper.registry.env.manifest_builder,
        ManifestBuilder::new(),
    );
    let calls = vec![HookCall::AfterRemoveLiquidity];
    helper.registry.env.manifest_builder = manifest_builder.call_function(
        package_address,
        "TestHook",
        "instantiate",
        manifest_args!(
            calls,
            TestAccess::new(),
            helper.x_address(),
            helper.y_address()
        ),
    );
    helper.registry.env.new_instruction("instantiate", 1, 0);

    let receipt = helper.registry.execute_expect_success(false);

    let new_resource_ads = receipt
        .execution_receipt
        .expect_commit_success()
        .new_resource_addresses();

    let outputs: Vec<(ComponentAddress, Bucket)> = receipt.outputs("instantiate");

    let hook_address = outputs[0].0;
    let hook_badge_address = new_resource_ads[0];

    let hook_infos = vec![(hook_address, hook_badge_address)];

    helper.instantiate_default_with_hooks(pdec!(1), hook_infos, false);
    helper
        .add_liquidity_default_batch(&ONE_LP)
        .registry
        .execute_expect_success(false);
    helper.removable_liquidity_success(
        lp_positions,
        x_output_expected,
        y_output_expected,
        minimum_removable_fraction_expected,
    );
}

/*
#[test]
fn test_receipt_output_buckets() {
    let mut helper: PoolTestHelper = PoolTestHelper::new();
    let receipt = helper.faucet_free().execute_success(false);
    let test_rri = ResourceAddress::try_from_hex(
        "010000000000000000000000000000000000000000000000000000"
    ).unwrap();
    assert_eq!(receipt.output_buckets("faucet_free"), vec![vec![Amount(test_rri, dec!(10000))]]);
}
*/

pub fn reverse_swap_type(swap_type: SwapType) -> SwapType {
    match swap_type {
        SwapType::BuyX => SwapType::SellX,
        SwapType::SellX => SwapType::BuyX,
    }
}
