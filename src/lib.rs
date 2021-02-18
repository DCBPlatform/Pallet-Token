#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, dispatch::DispatchResult,
	traits::{
		Currency, 
		ReservableCurrency, 
	},
};
use frame_system::{self as system, ensure_signed};
use parity_scale_codec::{Decode, Encode};
use sp_std::prelude::*;

#[cfg(test)]
mod tests;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type Currency: ReservableCurrency<Self::AccountId>;
}

pub type TokenIndex = u32;

type AccountIdOf<T> = <T as system::Trait>::AccountId;
type BalanceOf<T> = <<T as Trait>::Currency as Currency<AccountIdOf<T>>>::Balance;
type TokenInfoOf<T> = TokenInfo<AccountIdOf<T>, <T as system::Trait>::BlockNumber>;

#[derive(Encode, Decode, Default, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TokenInfo<AccountId, BlockNumber> {
	name: Vec<u8>,
	symbol: Vec<u8>,	
	owner: AccountId,
	created: BlockNumber,
}

decl_storage! {
	trait Store for Module<T: Trait> as TokenStore {

		pub Tokens get(fn tokens): map hasher(blake2_128_concat) TokenIndex => Option<TokenInfoOf<T>>;
		pub TokenCount get(fn token_count): TokenIndex;

		pub Balance get(fn balance): map hasher(blake2_128_concat) (u32, T::AccountId) => BalanceOf<T>;
		pub Supply get(fn supply): map hasher(blake2_128_concat) u32 => BalanceOf<T>;
		pub Paused get(fn paused): map hasher(blake2_128_concat) u32 => bool;
		pub Approval get(fn approval): map hasher(blake2_128_concat) (u32, T::AccountId, T::AccountId) => BalanceOf<T>;
		pub Owner get(fn owner): map hasher(blake2_128_concat) u32 => T::AccountId;
	}
}

decl_event!(
	pub enum Event<T>
	where
		AccountId = <T as system::Trait>::AccountId,
		Balance = BalanceOf<T>,
	{
		/// A token was created by user. \[token_id, owner_id\]
		Created(u32, AccountId),
		/// Token burned. \[token, sender, amount\]
		Burn(u32, AccountId, Balance),
		/// Token minted. \[token, receiver, amount\]
		Mint(u32, AccountId, Balance),
		/// Token transferred. \[token, sender, receiver, amount\]
		Transfer(u32, AccountId, AccountId, Balance),
		/// Token transferred. \[token, sender, spender, amount\]
		TransferFrom(u32, AccountId, AccountId, Balance),		
		/// Token approved. \[token, spender, user, amount\]
		Approval(u32, AccountId, AccountId, Balance),
		/// Token paused/unpaused. \[token, status\]
		PausedOperation(u32, bool),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
		NotTokenOwner,
		InsufficientAmount,
		InsufficientApproval,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;


		#[weight = 10_000]
		pub fn create(origin, 
			owner:AccountIdOf<T>, 
			name:Vec<u8>, 
			symbol: Vec<u8>, 
			initial_supply: BalanceOf<T>
		) -> DispatchResult {

			let caller = ensure_signed(origin)?;

			let index = TokenCount::get();
			TokenCount::put(index + 1);		
			
			let created = <system::Module<T>>::block_number();

			<Tokens<T>>::insert(index, TokenInfo {
				name,
				symbol,
				owner,
				created
			});			

			<Balance<T>>::insert((index, &caller), initial_supply);
			<Supply<T>>::insert(index, initial_supply);
			<Owner<T>>::insert(index, &caller);

			Self::deposit_event(RawEvent::Created(index, caller));

			Ok(())
		}	
		
		#[weight = 10_000]
		pub fn transfer(origin, 
			token:u32, 
			to: T::AccountId, 
			value: BalanceOf<T> 
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::transfer_(token, caller, to, value);
			Ok(())
		}	
		
		#[weight = 10_000]
		pub fn transfer_from(origin, 
			token:u32, 
			from: T::AccountId, 
			value: BalanceOf<T> 
		) -> DispatchResult {
			let to = ensure_signed(origin)?;
			Self::transfer_(token, from, to, value);
			Ok(())
		}			

		
		#[weight = 10_000]
		pub fn pause(origin, 
			token: u32, 
			status: bool 
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let token_owner = Self::owner(token);
			ensure!(caller == token_owner, <Error<T>>::NotTokenOwner);

			let token_boolean = Self::paused(token);
			let new_status: bool;
			if token_boolean {
				new_status = true;
			} else {	
				new_status = false;			
			}
			<Paused>::insert(token, new_status);			
			Self::deposit_event(RawEvent::PausedOperation(token, new_status));
			Ok(())
		}	
		
		#[weight = 10_000]
		pub fn mint(origin, 
			token:u32, 
			value: BalanceOf<T> 
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let token_owner = Self::owner(token);
			ensure!(caller == token_owner, <Error<T>>::NotTokenOwner);			
			Self::mint_(caller, token, value);
			Ok(())
		}	
		
		#[weight = 10_000]
		pub fn burn(origin, 
			token:u32, 
			value: BalanceOf<T> 
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let token_owner = Self::owner(token);
			ensure!(caller == token_owner, <Error<T>>::NotTokenOwner);			
			Self::burn_(caller, token, value);
			Ok(())
		}	

	
	}
}

impl<T: Trait> Module<T> {

	pub fn transfer_(token: u32, from: AccountIdOf<T>, to: AccountIdOf<T>, value: BalanceOf<T> ) -> () {
		let from_balance = Self::balance((token, &from));
		let to_balance = Self::balance((token, &to));

		<Balance<T>>::insert((token, &from), from_balance - value);
		<Balance<T>>::insert((token, &to), to_balance + value);
		Self::deposit_event(RawEvent::Transfer(token, from, to, value));
	}

	pub fn mint_(minter: AccountIdOf<T>, token: u32, value: BalanceOf<T>) -> () {
		let minter_balance = Self::balance((token, &minter));
		let token_supply = Self::supply(token);
		<Balance<T>>::insert((token, &minter), minter_balance + value);
		<Supply<T>>::insert(token, token_supply + value);

		Self::deposit_event(RawEvent::Mint(token, minter, value));
	}

	pub fn burn_(burner: AccountIdOf<T>, token: u32, value: BalanceOf<T>) -> () {
		let burner_balance = Self::balance((token, &burner));
		let token_supply = Self::supply(token);

		<Balance<T>>::insert((token, &burner), burner_balance - value);
		<Supply<T>>::insert(token, token_supply - value);

		Self::deposit_event(RawEvent::Burn(token, burner, value));
	}	

	pub fn get_balance(token: u32, who: AccountIdOf<T> ) -> BalanceOf<T> {
		Self::balance((token, who))
	}		


}
