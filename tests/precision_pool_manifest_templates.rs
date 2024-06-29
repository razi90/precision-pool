#[cfg(test)]
mod precision_pool_manifest_templates {
    use std::mem;

    // INSTANTIATE
    use precision_pool_test_helper::*;
    use scrypto::prelude::*;
    use scrypto_test::utils::dump_manifest_to_file_system;
    use scrypto_testenv::nft_ids;

    #[test]
    fn test_dump_instantiate() {
        let mut helper: PoolTestHelper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.instantiate(
            helper.x_address(),
            helper.y_address(),
            pdec!(1.1),
            dec!(0.003),
            dec!(0.009),
            helper.registry.registry_address.unwrap(),
            vec![],
        );
        let manifest_builder = mem::take(&mut helper.registry.env.manifest_builder)
            .deposit_batch(helper.registry.env.account);
        dump_manifest_to_file_system(
            manifest_builder.object_names(),
            &manifest_builder.build(),
            "./transaction-manifest",
            Some("instantiate"),
            &NetworkDefinition::simulator(),
        )
        .err();
    }

    #[test]
    fn test_dump_instantiate_with_liquidity() {
        let mut helper: PoolTestHelper = PoolTestHelper::new();
        helper
            .registry
            .instantiate_default(helper.registry.admin_badge_address());
        helper.instantiate_with_liquidity(
            helper.x_address(),
            dec!(20),
            helper.y_address(),
            dec!(30),
            pdec!(1.1),
            dec!(0.003),
            helper.registry.registry_address.unwrap(),
            vec![],
            -10000,
            15000,
        );
        let manifest_builder = mem::take(&mut helper.registry.env.manifest_builder)
            .deposit_batch(helper.registry.env.account);
        dump_manifest_to_file_system(
            manifest_builder.object_names(),
            &manifest_builder.build(),
            "./transaction-manifest",
            Some("instantiate_with_liquidity"),
            &NetworkDefinition::simulator(),
        )
        .err();
    }

    #[test]
    fn test_dump_add_liquidity() {
        let mut helper: PoolTestHelper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1), true);
        helper.add_liquidity_default(2000, 5000, dec!(20), dec!(30));
        let manifest_builder = mem::take(&mut helper.registry.env.manifest_builder)
            .deposit_batch(helper.registry.env.account);
        dump_manifest_to_file_system(
            manifest_builder.object_names(),
            &manifest_builder.build(),
            "./transaction-manifest",
            Some("add_liquidity"),
            &NetworkDefinition::simulator(),
        )
        .err();
    }

    #[test]
    fn test_dump_swap() {
        let mut helper: PoolTestHelper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1), true);
        helper.swap(helper.x_address(), dec!(5));
        let manifest_builder = mem::take(&mut helper.registry.env.manifest_builder)
            .deposit_batch(helper.registry.env.account);
        dump_manifest_to_file_system(
            manifest_builder.object_names(),
            &manifest_builder.build(),
            "./transaction-manifest",
            Some("swap"),
            &NetworkDefinition::simulator(),
        )
        .err();
    }

    #[test]
    fn test_dump_remove_liquidity() {
        let mut helper: PoolTestHelper = PoolTestHelper::new();
        helper.instantiate_default(pdec!(1), true);
        helper.remove_liquidity(nft_ids!(1));
        let manifest_builder = mem::take(&mut helper.registry.env.manifest_builder)
            .deposit_batch(helper.registry.env.account);
        dump_manifest_to_file_system(
            manifest_builder.object_names(),
            &manifest_builder.build(),
            "./transaction-manifest",
            Some("remove_liquidity"),
            &NetworkDefinition::simulator(),
        )
        .err();
    }
}
