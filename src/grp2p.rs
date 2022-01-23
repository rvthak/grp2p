#![no_std]

elrond_wasm::imports!();

mod order;
use order::SwapOrder;

// The default swap fee - set on init
const DEFAULT_FEE: u32 = 10;
const PERCENT_BASE_POINTS: u64 = 1_000;

#[elrond_wasm::derive::contract]
pub trait Grp2p {

    #[init]
    fn init(
        &self,
        token1: TokenIdentifier,
        token2: TokenIdentifier,
    ) -> SCResult<()> {

        // Initialize the order-id counter
        self.id().set(&BigUint::zero());

        // Ensure that the given tokens are valid
        require!( token1.is_valid_esdt_identifier() && token2.is_valid_esdt_identifier(), "Invalid token given");

        // Init the claimable fees of the given tokens to zero
        self.claimable_fee(&token1).set(&BigUint::zero());
        self.claimable_fee(&token2).set(&BigUint::zero());

        // Add the two initial tokens in the whitelist
        self.whitelist().insert(token1);
        self.whitelist().insert(token2);

        // Initialize the swap fee
        self.fee().set(&DEFAULT_FEE);
        Ok(()) 
    }

    // User creates a new pair swap by providing his token amount and
    // requesting an amount of another token in return
    #[endpoint]
    #[payable("*")]
    fn create(
        &self,
        #[payment_token] provided_token: TokenIdentifier,
        #[payment] provided_amount: BigUint,
        requested_token: TokenIdentifier,
        requested_amount: BigUint,
    ) -> SCResult<()> {

        // Check if the tokens are whitelisted
        self.is_whitelisted(&provided_token)?;
        self.is_whitelisted(&requested_token)?;

        // Ensure that the tokens are not the same
        require!(provided_token != requested_token, "Same token provided and requested");

        // Increment the order id counter and get an id 
        self.increment();
        let new_id = self.id().get();

        let order_id = new_id.clone();
        let id_c1 = new_id.clone();
        let id_c2 = new_id.clone();
        let id_c3 = new_id.clone();

        // Get the order maker's address
        let maker = self.blockchain().get_caller();

        // Add the ID to the corresponding buy/sell lists
        self.buy_orders(&requested_token).insert(id_c1);
        self.sell_orders(&provided_token).insert(id_c2);

        // Add the ID to the user's order list
        self.user_orders(&maker).insert(id_c3);

        // Store the newly created order
        let order = SwapOrder {
            order_id,
            maker,
            provided_token,
            requested_token,
            provided_amount,
            requested_amount,
        };
        self.orders(&new_id).set(&order);

        Ok(())
    }
    

    // User cancels an existing pair swap that he has created but has not
    // yet been satisfied
    #[endpoint]
    fn cancel(
        &self,
        order_id: BigUint,
    ) -> SCResult<()> {

        // Check if there is an order with the given id stored
        require!( !self.orders(&order_id).is_empty(), "Invalid order id" );

        let order = self.orders(&order_id).get();

        // Ensure caller is the order maker
        require!( self.blockchain().get_caller() == order.maker , "Permission denied" );

        // Return the funds
        self.send().direct(
            &order.maker,
            &order.provided_token,
            0,
            &order.provided_amount,
            b"Order successfully canceled",
        );

        // Delete the stored order from all places
        self.buy_orders(&order.requested_token).remove(&order_id);
        self.sell_orders(&order.provided_token).remove(&order_id);
        self.user_orders(&order.maker).remove(&order_id);
        self.orders(&order_id).clear();

        Ok(())
    }  
    

    // User accepts an existing pair swap and sends the required token amount
    // to complete the token swap
    #[endpoint]
    #[payable("*")]
    fn swap(
        &self,
        #[payment_token] provided_token: TokenIdentifier,
        #[payment] provided_amount: BigUint,
        order_id: BigUint,
    ) -> SCResult<()> {

        // Check if there is an order with the given id stored
        require!( !self.orders(&order_id).is_empty(), "Invalid order id" );

        let order = self.orders(&order_id).get();
        let maker = &order.maker;
        let taker = self.blockchain().get_caller();

        // Ensure caller is not maker
        require!( taker != *maker, "Invalid caller: maker" );

        // Ensure that the payment is correct
        require!(order.requested_token == provided_token && order.requested_amount == provided_amount, "Swap Failed: Wrong token/amount provided");

        // Receive a small fee from both token amounts
        let prov_fee = self.calculate_fee_amount(&order.provided_amount);
        let requ_fee = self.calculate_fee_amount(&order.requested_amount);

        // Store fee amount
        self.store_claimable(&order.provided_token , &prov_fee);
        self.store_claimable(&order.requested_token, &requ_fee);

        // Calculate the final token amounts
        let prov_amount = order.provided_amount  - prov_fee;
        let requ_amount = order.requested_amount - requ_fee;

        // Send tokens to both users
        self.send().direct(
            &maker,
            &order.requested_token,
            0,
            &requ_amount,
            b"Swap successfully completed",
        );

        self.send().direct(
            &taker,
            &order.provided_token,
            0,
            &prov_amount,
            b"Swap successfully completed",
        );

        // Delete the stored order
        self.buy_orders(&order.requested_token).remove(&order_id);
        self.sell_orders(&order.provided_token).remove(&order_id);
        self.user_orders(&order.maker).remove(&order_id);
        self.orders(&order_id).clear();
        Ok(())
    }

    // ------------------------- Owner ------------------------- 

    // Allow the owner to claim the fees of a given token
    #[endpoint]
    fn claim_fees(&self, token: TokenIdentifier) -> SCResult<()> {

        // Ensure caller is owner
        self.require_permissions()?;

        // Ensure there are fees to be claimed on this token
        require!(!self.claimable_fee(&token).is_empty(), "No fees available for claiming");

        // Send the fees to the owner
        let avail = self.claimable_fee(&token).get();
        let owner = self.blockchain().get_owner_address();

        self.send().direct(
            &owner,
            &token,
            0,
            &avail,
            b"Fees claimed successfully!",
        );
        // ! ONLY the token amount that is actually a fee NOT the contract needed tokens

        // Zero the fee balance
        self.claimable_fee(&token).set(&BigUint::zero());

        Ok(())
    }

    // Update the current swap fee (out of 1000)
    #[endpoint]
    fn update_fee(&self, new_fee : &u32) -> SCResult<()> {

        // Ensure caller is owner
        self.require_permissions()?;

        // Set the new fee
        self.fee().set(&new_fee);
        Ok(())
    }

    // Add a new token on the token whitelist
    #[endpoint]
    fn add_token(&self, token: TokenIdentifier) -> SCResult<()> {

        // Ensure caller is owner
        self.require_permissions()?;

        // Ensure that the given token is valid
        require!(token.is_valid_esdt_identifier(), "Invalid token given");

        // Add the token to the token whitelist
        self.whitelist().insert(token); // Returns false in case the token is already in the set
        Ok(())
    }

    // Remove a token from the token whitelist
    #[endpoint]
    fn remove_token(&self, token: &TokenIdentifier) -> SCResult<()> {

        // Ensure caller is owner
        self.require_permissions()?;

        // Remove the token from the whitelist
        self.whitelist().remove(&token);

        // IMPORTANT: We Don't clear any existing swap orders pending
        // for this token. We just removed the user's ability to
        // create new orders but we can't cancel the existing ones
        Ok(())
    }

    // Allows the Owner to see the amount of a specific token that is available to claim
    #[endpoint]
    fn available_fees(&self, token: &TokenIdentifier) -> SCResult<BigUint> {
        // Ensure caller is owner
        self.require_permissions()?;

        // Get the stored amount for the specific token
        return elrond_wasm::types::SCResult::Ok(self.claimable_fee(&token).get());
    } // Returns zero in case the token does not exist

    // --------------------------- API --------------------------- 

    // Get all the active orders of a single user given his address
    #[endpoint]
    fn get_user_orders(&self) -> SCResult<Vec<SwapOrder<Self::Api>>> {
        let caller = self.blockchain().get_caller();
        let mut ordlist = Vec::new();

        // For each one of the caller's orders
        for order_id in self.user_orders(&caller).iter() {
            // Get the order data based on the id and add it to the vector
            let order = self.orders(&order_id).get();
            ordlist.push(order);
        }
        return elrond_wasm::types::SCResult::Ok(ordlist);
    }

    // Get all active orders that want to buy the provided token
    #[endpoint]
    fn get_token_buy_orders(&self, token: TokenIdentifier) -> SCResult<Vec<SwapOrder<Self::Api>>> {
        let mut ordlist = Vec::new();

        // For each one of the caller's orders
        for order_id in self.buy_orders(&token).iter() {
            // Get the order data based on the id and add it to the vector
            let order = self.orders(&order_id).get();
            ordlist.push(order);
        }
        return elrond_wasm::types::SCResult::Ok(ordlist);
    }

    // Get all active orders that want to sell the requested token
    #[endpoint]
    fn get_token_sell_orders(&self, token: TokenIdentifier) -> SCResult<Vec<SwapOrder<Self::Api>>> {
        let mut ordlist = Vec::new();

        // For each one of the caller's orders
        for order_id in self.sell_orders(&token).iter() {
            // Get the order data based on the id and add it to the vector
            let order = self.orders(&order_id).get();
            ordlist.push(order);
        }
        return elrond_wasm::types::SCResult::Ok(ordlist);
    }

    // Returns the current fee
    #[endpoint]
    fn get_fee(&self) -> SCResult<u32> {

        return elrond_wasm::types::SCResult::Ok(self.fee().get());
    }

    // Check if the token in question is whitelisted or not
    #[endpoint]
    fn is_whitelisted(&self, token: &TokenIdentifier) -> SCResult<()> {
        require!(self.whitelist().contains(&token), "Token not whitelisted");
        Ok(())
    }

    // Return a Vector containing all the supported tokens
    #[endpoint]
    fn get_whitelisted(&self) -> SCResult<Vec<TokenIdentifier>> {
        // Simply Create a vector and add all the stored tokens to it
        let mut whitelisted = Vec::new();
        for token in self.whitelist().iter() {
            whitelisted.push(token);
        }
        return elrond_wasm::types::SCResult::Ok(whitelisted);
    }

    // -------------------------- Utils -------------------------- 

    // Update the stored amount of fees for the given token
    fn store_claimable(&self, token:&TokenIdentifier, amount:&BigUint){
        let cur = self.claimable_fee(token).get();
        let new = cur + amount;
        self.claimable_fee(token).set( &new );
    }

    // Calculate the amount of fees
    fn calculate_fee_amount(&self, amount: &BigUint) -> BigUint {
        let fee_per = self.fee().get();
        return amount.mul(fee_per) / PERCENT_BASE_POINTS;
    }

    // Add the given value to the stored id value
    fn increment(&self){ 
        let adder : u32 = 1;
        self.id().update(|id| *id += adder ); 
    }

    // Check if the caller has owner permissions
    fn require_permissions(&self) -> SCResult<()> {
        require!(self.blockchain().get_caller() == self.blockchain().get_owner_address(), "Permission denied");
        Ok(())
    }

    // ------------------------- Storage ------------------------- 

    // The swap order-id counter
    #[storage_mapper("id")]
    fn id(&self) -> SingleValueMapper<BigUint>;

    // A supported esdt token whitelist
    #[storage_mapper("whitelist")]
    fn whitelist(&self) -> SetMapper<TokenIdentifier>;

    // The current fee percentage
    #[storage_mapper("fee")]
    fn fee(&self) -> SingleValueMapper<u32>;

    // Store the currently available claimable fees
    #[storage_mapper("claimable")]
    fn claimable_fee(&self, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    // Stores the swap orders themselves (order_id : SwapOrder)
    #[storage_mapper("orders")]
    fn orders(&self, id: &BigUint) -> SingleValueMapper<SwapOrder<Self::Api>>;

    // Stores the ids of buy orders for the given token
    #[storage_mapper("buy_orders")]
    fn buy_orders(&self, token: &TokenIdentifier) -> SetMapper<BigUint>;

    // Stores the ids of sell orders for the given token
    #[storage_mapper("sell_orders")]
    fn sell_orders(&self, token: &TokenIdentifier) -> SetMapper<BigUint>;

    // Stores the ids of orders for the given user
    #[storage_mapper("user_orders")]
    fn user_orders(&self, user: &ManagedAddress) -> SetMapper<BigUint>;
}
