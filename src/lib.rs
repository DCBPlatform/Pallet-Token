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
	type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
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
	trait Store for Module<T: Trait> as Token {

		pub Tokens get(fn tokens): map hasher(blake2_128_concat) TokenIndex => Option<TokenInfoOf<T>>;
		pub TokenCount get(fn token_count): TokenIndex;

		pub TokenBalance get(fn token_balance): map hasher(blake2_128_concat) (u32, T::AccountId) => BalanceOf<T>;
		pub TokenSupply get(fn token_supply): map hasher(blake2_128_concat) u32 => BalanceOf<T>;
		pub TokenPaused get(fn token_paused): map hasher(blake2_128_concat) u32 => bool;
		pub TokenApproval get(fn token_approval): map hasher(blake2_128_concat) (u32, T::AccountId, T::AccountId) => BalanceOf<T>;
		pub TokenOwner get(fn token_owner): map hasher(blake2_128_concat) u32 => T::AccountId;
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
		fn create(origin, 
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

			<TokenBalance<T>>::insert((index, &caller), initial_supply);
			<TokenSupply<T>>::insert(index, initial_supply);
			<TokenOwner<T>>::insert(index, &caller);

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

			let sender_balance = Self::token_balance((token, &caller));
			let receiver_balance = Self::token_balance((token, &to));

			<TokenBalance<T>>::insert((token, &caller), sender_balance - value);
			<TokenBalance<T>>::insert((token, &to), receiver_balance + value);

			Self::deposit_event(RawEvent::Transfer(token, caller, to, value));
			Ok(())
		}	
		
		#[weight = 10_000]
		pub fn transfer_from(origin, 
			token:u32, 
			from: T::AccountId, 
			value: BalanceOf<T> 
		) -> DispatchResult {
			let to = ensure_signed(origin)?;

			let from_balance = Self::token_balance((token, &from));
			let to_balance = Self::token_balance((token, &to));

			<TokenBalance<T>>::insert((token, &from), from_balance - value);
			<TokenBalance<T>>::insert((token, &to), to_balance + value);

			Self::deposit_event(RawEvent::TransferFrom(token, from, to, value));
			Ok(())
		}			

		
		#[weight = 10_000]
		fn pause(origin, 
			token: u32, 
			status: bool 
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let token_owner = Self::token_owner(token);
			ensure!(caller == token_owner, <Error<T>>::NotTokenOwner);

			let token_boolean = Self::token_paused(token);
			let new_status: bool;
			if token_boolean {
				new_status = true;
			} else {	
				new_status = false;			
			}
			<TokenPaused>::insert(token, new_status);			
			Self::deposit_event(RawEvent::PausedOperation(token, new_status));
			Ok(())
		}	
		
		#[weight = 10_000]
		fn mint(origin, 
			token:u32, 
			value: BalanceOf<T> 
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let token_owner = Self::token_owner(token);
			ensure!(caller == token_owner, <Error<T>>::NotTokenOwner);			

			let minter_balance = Self::token_balance((token, &caller));
			let token_supply = Self::token_supply(token);

			<TokenBalance<T>>::insert((token, &caller), minter_balance + value);
			<TokenSupply<T>>::insert(token, token_supply + value);

			Self::deposit_event(RawEvent::Mint(token, caller, value));
			Ok(())
		}	
		
		#[weight = 10_000]
		fn burn(origin, 
			token:u32, 
			value: BalanceOf<T> 
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			let token_owner = Self::token_owner(token);
			ensure!(caller == token_owner, <Error<T>>::NotTokenOwner);			

			let burner_balance = Self::token_balance((token, &caller));
			let token_supply = Self::token_supply(token);

			<TokenBalance<T>>::insert((token, &caller), burner_balance - value);
			<TokenSupply<T>>::insert(token, token_supply - value);

			Self::deposit_event(RawEvent::Burn(token, caller, value));
			Ok(())
		}				
		


	}
}

impl<T: Trait> Module<T> {

	pub fn transfer_(token: u32, from: AccountIdOf<T>, to: AccountIdOf<T>, value: BalanceOf<T> ) -> () {
		let from_balance = Self::token_balance((token, &from));
		let to_balance = Self::token_balance((token, &to));

		<TokenBalance<T>>::insert((token, &from), from_balance - value);
		<TokenBalance<T>>::insert((token, &to), to_balance + value);
	}
}
