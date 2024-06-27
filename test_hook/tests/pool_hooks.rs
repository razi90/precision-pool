mod helper;

#[cfg(test)]
mod test_hook_precision_pool {
    use super::*;
    use precision_pool_hooks::HookCall;
    use helper::HookTestTestHelper;
    use scrypto::prelude::*;
    use test_hook::test_hook::TestAccess;

    #[test]
    fn test_all_calls_all_accesses() {
        let mut hook_helper = HookTestTestHelper::new();

        // Instantiate hook
        let mut hooks = Vec::new();
        let mut calls = Vec::new();
        calls.push(HookCall::BeforeInstantiate);
        calls.push(HookCall::AfterInstantiate);
        calls.push(HookCall::BeforeSwap);
        calls.push(HookCall::AfterSwap);
        calls.push(HookCall::BeforeAddLiquidity);
        calls.push(HookCall::AfterAddLiquidity);
        calls.push(HookCall::BeforeRemoveLiquidity);
        calls.push(HookCall::AfterRemoveLiquidity);

        let access = TestAccess::new();

        hooks.push(hook_helper.instantiate_test_hook_output(calls, access));

        hook_helper.execute_all_calls(hooks);
    }

    #[test]
    fn test_all_calls_without_hooks() {
        let mut hook_helper = HookTestTestHelper::new();

        // Instantiate hook
        let mut hooks = Vec::new();
        let calls = Vec::new();

        let access = TestAccess::new();

        hooks.push(hook_helper.instantiate_test_hook_output(calls, access));

        hook_helper.execute_all_calls(hooks);
    }

    #[test]
    fn test_all_calls_with_two_same_hooks() {
        let mut hook_helper = HookTestTestHelper::new();

        // Instantiate hook
        let mut hooks = Vec::new();
        let mut calls = Vec::new();

        calls.push(HookCall::BeforeInstantiate);
        calls.push(HookCall::AfterInstantiate);
        calls.push(HookCall::BeforeSwap);
        calls.push(HookCall::AfterSwap);
        calls.push(HookCall::BeforeAddLiquidity);
        calls.push(HookCall::AfterAddLiquidity);
        calls.push(HookCall::BeforeRemoveLiquidity);
        calls.push(HookCall::AfterRemoveLiquidity);

        let access = TestAccess::new();

        // First test hook
        hooks.push(hook_helper.instantiate_test_hook_output(calls.clone(), access.clone()));
        // Second test hook
        hooks.push(hook_helper.instantiate_test_hook_output(calls, access));

        hook_helper.execute_all_calls(hooks);
    }

    #[test]
    fn test_all_calls_not_x_output_in_remove() {
        let mut hook_helper = HookTestTestHelper::new();

        // Instantiate hook
        let mut hooks = Vec::new();
        let mut calls = Vec::new();
        calls.push(HookCall::BeforeInstantiate);
        calls.push(HookCall::AfterInstantiate);
        calls.push(HookCall::BeforeSwap);
        calls.push(HookCall::AfterSwap);
        calls.push(HookCall::BeforeAddLiquidity);
        calls.push(HookCall::AfterAddLiquidity);
        calls.push(HookCall::BeforeRemoveLiquidity);
        calls.push(HookCall::AfterRemoveLiquidity);

        let mut access = TestAccess::new();
        access.after_remove_liquidity_x_output = false;

        hooks.push(hook_helper.instantiate_test_hook_output(calls, access));

        hook_helper.execute_all_calls(hooks);
    }

    #[test]
    fn test_add_call_twice() {
        let mut hook_helper = HookTestTestHelper::new();

        // Instantiate hook
        let mut hooks = Vec::new();
        let mut calls = Vec::new();
        calls.push(HookCall::BeforeInstantiate);
        calls.push(HookCall::AfterInstantiate);
        calls.push(HookCall::BeforeSwap);
        calls.push(HookCall::AfterSwap);
        calls.push(HookCall::BeforeAddLiquidity);
        calls.push(HookCall::AfterAddLiquidity);
        calls.push(HookCall::BeforeRemoveLiquidity);
        calls.push(HookCall::AfterRemoveLiquidity);
        calls.push(HookCall::AfterRemoveLiquidity);

        let access = TestAccess::new();

        hooks.push(hook_helper.instantiate_test_hook_output(calls, access));

        hook_helper.execute_all_calls(hooks);
    }
}
