mod aurora;

use aurora_engine_types::ethabi::{self, Token};
use aurora_engine_types::types::{RawAddress, RawU256};
use aurora_engine_types::Address;
use aurora_engine_types::U256;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, WrappedBalance};
use near_sdk::{
    env, ext_contract, log, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault,
    Promise, PromiseOrValue,
};

use crate::aurora::*;

near_sdk::setup_alloc!();

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    Erc20ToNep141,
    Nep141ToErc20,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    erc20_to_nep141: LookupMap<RawAddress, AccountId>,
    nep141_to_erc20: LookupMap<AccountId, RawAddress>,
}

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn on_balance(&self);
    fn on_near_erc20_token_balance(&self);
    #[result_serializer(borsh)]
    fn on_get_erc20_from_nep141(&self, #[serializer(borsh)] token_id: AccountId);
}

pub trait ExtSelf {
    fn on_balance(&self, balance: RawU256) -> WrappedBalance;
    fn on_near_erc20_token_balance(&self, tx_result: TransactionStatus) -> Option<WrappedBalance>;
    fn on_get_erc20_from_nep141(
        &mut self,
        token_address: RawAddress,
        token_id: AccountId,
    ) -> String;
}

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    pub fn new() -> Self {
        Self {
            erc20_to_nep141: LookupMap::new(StorageKey::Erc20ToNep141),
            nep141_to_erc20: LookupMap::new(StorageKey::Nep141ToErc20),
        }
    }

    pub fn near_address(&self) -> String {
        let address = near_account_to_evm_address(&env::current_account_id());
        address_to_string(&address)
    }

    pub fn get_native_balance(&self) -> Promise {
        let address = near_account_to_evm_address(&env::current_account_id());
        ext_aurora::get_balance(address).then(ext_self::on_balance(
            &env::current_account_id(),
            NO_DEPOSIT,
            5 * TGAS,
        ))
    }

    pub fn get_near_erc20_token_balance(&self, token_id: ValidAccountId) -> Promise {
        let address = near_account_to_evm_address(&env::current_account_id());
        let token_address = self.get_erc20_address(token_id.as_ref());
        ext_aurora::get_erc20_balance(token_address, address).then(
            ext_self::on_near_erc20_token_balance(
                &env::current_account_id(),
                NO_DEPOSIT,
                10 * TGAS,
            ),
        )
    }

    pub fn fetch_erc20_token_address(
        &mut self,
        token_id: ValidAccountId,
    ) -> PromiseOrValue<String> {
        if let Some(raw_address) = self.nep141_to_erc20.get(token_id.as_ref()) {
            return PromiseOrValue::Value(address_to_string(&Address(raw_address)));
        }
        ext_aurora::get_erc20_from_nep141(token_id.as_ref())
            .then(ext_self::on_get_erc20_from_nep141(
                token_id.into(),
                &env::current_account_id(),
                NO_DEPOSIT,
                10 * TGAS,
            ))
            .into()
    }

    #[private]
    pub fn erc20_transfer(
        &mut self,
        token_id: ValidAccountId,
        receiver: String,
        amount: WrappedBalance,
    ) -> Promise {
        let token_address = self.get_erc20_address(token_id.as_ref());
        let receiver = parse_address(&receiver);
        log!("Sending {} to {}", amount.0, address_to_string(&receiver));
        ext_aurora::erc20_transfer_balance(token_address, receiver, amount.0)
    }
}

impl Contract {
    fn get_erc20_address(&self, token_id: &AccountId) -> Address {
        let token_address = Address(
            self.nep141_to_erc20
                .get(&token_id)
                .expect("Need to fetch token first."),
        );
        log!("Token Address: {}", address_to_string(&token_address));
        token_address
    }
}

#[near_bindgen]
impl ExtSelf for Contract {
    #[private]
    fn on_balance(
        &self,
        #[callback]
        #[serializer(borsh)]
        balance: RawU256,
    ) -> WrappedBalance {
        let balance = U256::from(balance).as_u128();
        log!("Your balance is {}", balance);
        balance.into()
    }

    #[private]
    fn on_near_erc20_token_balance(
        &self,
        #[callback]
        #[serializer(borsh)]
        tx_result: TransactionStatus,
    ) -> Option<WrappedBalance> {
        match tx_result {
            TransactionStatus::Succeed(res) => {
                let balance = U256::from(res.as_slice()).as_u128();
                log!("Your token balance is {}", balance);
                Some(balance.into())
            }
            _ => {
                log!("Failed to fetch token balance");
                None
            }
        }
    }

    #[private]
    fn on_get_erc20_from_nep141(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        token_address: RawAddress,
        #[serializer(borsh)] token_id: AccountId,
    ) -> String {
        self.erc20_to_nep141.insert(&token_address, &token_id);
        self.nep141_to_erc20.insert(&token_id, &token_address);
        address_to_string(&Address(token_address))
    }
}
