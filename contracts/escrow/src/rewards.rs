use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{Address, Bytes, BytesN, Env};

pub const REWARD_LEAF_PREFIX: &[u8] = &[0x00];

pub fn compute_leaf_hash(e: &Env, vault: &Address, epoch: u32, deposit_id: u64, owner: &Address, amount: u128) -> BytesN<32> {
    let mut payload = Bytes::from_slice(e, REWARD_LEAF_PREFIX);
    let encoded = (vault.clone(), epoch, deposit_id, owner.clone(), amount).to_xdr(e);
    payload.append(&encoded);
    e.crypto().sha256(&payload).into()
}

pub fn compute_root_from_proof(e: &Env, leaf: &BytesN<32>, proof: &soroban_sdk::Vec<BytesN<32>>, mut index: u32) -> BytesN<32> {
    let mut hash = leaf.clone();

    for sibling in proof.iter() {
        let sibling_bytes = sibling.to_array();
        let current_bytes = hash.to_array();

        let mut data = [0u8; 65];
        data[0] = 0x01;

        if index % 2 == 0 {
            data[1..33].copy_from_slice(&current_bytes);
            data[33..65].copy_from_slice(&sibling_bytes);
        } else {
            data[1..33].copy_from_slice(&sibling_bytes);
            data[33..65].copy_from_slice(&current_bytes);
        }

        let buffer = Bytes::from_slice(e, &data);
        hash = e.crypto().sha256(&buffer).into();
        index /= 2;
    }

    hash
}
