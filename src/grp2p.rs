#![no_std]

elrond_wasm::imports!();

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


    // ------------------------- Owner ------------------------- 

    // // Add a new token on the token whitelist
    // #[endpoint]
    // fn add_token(
    //     &self,
    //     token: TokenIdentifier,
    // ) -> SCResult<()> {

    //     // Ensure caller is owner
    //     self.require_permissions()?;

    //     // Add the token to the token whitelist
    //     self.whitelist().push(&token);

    //     Ok(())
    // }

    // // Remove a token from the token whitelist
    // #[endpoint]
    // fn remove_token(
    //     &self,
    //     token: TokenIdentifier,
    // ) -> SCResult<()> {

    //     // Ensure caller is owner
    //     self.require_permissions()?;

    //     // Remove the token from the whitelist
    //     self.whitelist().pop(&token);

    //     // IMPORTANT: Don't clear any existing swap orders pending
    //     // for this token. We just removed the user's ability to
    //     // create new orders but we can't cancel the existing ones

    //     Ok(())
    // }


    // -------------------------- Utils -------------------------- 

    fn add(&self, value : &u32){ self.id().update(|id| (*id) += *value ); }

    // ------------------------- Storage ------------------------- 

    // The swap order-id counter
    #[storage_mapper("id")]
    fn id(&self) -> SingleValueMapper<BigUint>;

    // A supported esdt token whitelist
    #[storage_mapper("whitelist")]
    fn whitelist(&self) -> SetMapper<TokenIdentifier>;

    // Store and retrieve the contract's owner
    #[storage_set("owner")]
    fn set_owner(&self, address: &ManagedAddress);

    #[storage_get("owner")]
    fn get_owner(&self) -> ManagedAddress;
}
