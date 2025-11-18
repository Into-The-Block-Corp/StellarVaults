#![cfg(test)]

use crate::contract::{EscrowContract, EscrowContractClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Env};

pub struct TestData<'a> {
    pub contract: EscrowContractClient<'a>,
    pub admin: Address,
    pub vault_a: Address,
    pub token_a: Address,
    pub token_a_tc: TokenClient<'a>,
    pub token_a_sac: StellarAssetClient<'a>,
    pub token_b: Address,
    pub token_b_tc: TokenClient<'a>,
    pub token_b_sac: StellarAssetClient<'a>,
}

pub fn create_test_data(e: &Env) -> TestData {
    let admin: Address = Address::generate(&e);
    let token_a: Address = e.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_b: Address = e.register_stellar_asset_contract_v2(admin.clone()).address();

    let contract_id: Address = e.register(EscrowContract, (admin.clone(),));

    let contract: EscrowContractClient = EscrowContractClient::new(&e, &contract_id);

    let token_a_tc: TokenClient = TokenClient::new(&e, &token_a);
    let token_a_sac: StellarAssetClient = StellarAssetClient::new(&e, &token_a);

    let token_b_tc: TokenClient = TokenClient::new(&e, &token_b);
    let token_b_sac: StellarAssetClient = StellarAssetClient::new(&e, &token_b);

    let vault_a = Address::generate(&e);

    TestData {
        contract,
        admin,
        vault_a,
        token_a,
        token_a_tc,
        token_a_sac,
        token_b,
        token_b_tc,
        token_b_sac,
    }
}
