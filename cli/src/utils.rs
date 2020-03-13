use ckb_tool::ckb_crypto::secp::{Privkey, Pubkey};
use ckb_tool::ckb_hash::new_blake2b;
use ckb_tool::ckb_types::H160;
use rand::Rng;

pub fn random_privkey_seed() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut seed = [0u8; 32];
    loop {
        rng.fill(&mut seed);
        let privkey = Privkey::from_slice(&seed);
        // test our seed is valid
        if let Ok(_) = privkey.pubkey() {
            return seed;
        }
    }
}

pub fn pubkey_hash(pubkey: &Pubkey) -> H160 {
    let mut hasher = new_blake2b();
    hasher.update(&pubkey.serialize());
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    let mut hash160 = [0u8; 20];
    hash160.copy_from_slice(&hash[..20]);
    hash160.into()
}
