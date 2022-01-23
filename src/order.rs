
elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(NestedEncode, NestedDecode, TopEncode, TopDecode, TypeAbi)]
pub struct SwapOrder<M: ManagedTypeApi>{
	pub order_id: BigUint<M>,
	pub maker: ManagedAddress<M>,
	pub provided_token: TokenIdentifier<M>,
	pub requested_token: TokenIdentifier<M>,
	pub provided_amount: BigUint<M>,
	pub requested_amount: BigUint<M>,
}
