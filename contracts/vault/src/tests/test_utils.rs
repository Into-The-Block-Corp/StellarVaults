#![cfg(test)]

use crate::contract::{VaultContract, VaultContractClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Env};

pub struct TestData<'a> {
    pub contract: VaultContractClient<'a>,
    pub admin: Address,
    pub deposit_asset: Address,
    pub deposit_asset_tc: TokenClient<'a>,
    pub deposit_asset_sac: StellarAssetClient<'a>,
}

pub fn create_test_data(e: &Env) -> TestData {
    let admin: Address = Address::generate(&e);
    let deposit_asset: Address = e.register_stellar_asset_contract_v2(admin.clone()).address();

    let contract_id: Address = e.register(VaultContract, (admin.clone(), deposit_asset.clone()));

    let contract: VaultContractClient = VaultContractClient::new(&e, &contract_id);

    let deposit_asset_tc: TokenClient = TokenClient::new(&e, &deposit_asset);
    let deposit_asset_sac: StellarAssetClient = StellarAssetClient::new(&e, &deposit_asset);

    TestData {
        contract,
        admin,
        deposit_asset,
        deposit_asset_tc,
        deposit_asset_sac,
    }
}
