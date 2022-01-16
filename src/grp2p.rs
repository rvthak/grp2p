#![no_std]

elrond_wasm::imports!();

/// One of the simplest smart contracts possible,
/// it holds a single variable in storage, which anyone can increment.
#[elrond_wasm::derive::contract]
pub trait Grp2p {

    #[init]
    fn init(&self){

        // Start by storing the owner's address
        let my_address: ManagedAddress = self.blockchain().get_caller();
        self.set_owner(&my_address);

        // Initialize the order-id counter
        let counter = BigUint::zero();
        self.id().set(&counter);
    }

    // -------------------------- Utils -------------------------- 

    fn add(&self, value : &u32){ self.id().update(|id| (*id) += *value ); }

    // ------------------------- Storage ------------------------- 

    #[storage_mapper("id")]
    fn id(&self) -> SingleValueMapper<BigUint>;

    #[storage_set("owner")]
    fn set_owner(&self, address: &ManagedAddress);

    #[storage_get("owner")]
    fn get_owner(&self) -> ManagedAddress;
}
