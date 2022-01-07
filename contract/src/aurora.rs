use near_sdk::{AccountId, Gas};
use primitive_types::H256;

use crate::*;

pub const AURORA: &str = "aurora";
pub const TGAS: Gas = 10u64.pow(12);
pub const VIEW_CALL_GAS: Gas = 100 * TGAS;
pub const CALL_GAS: Gas = 200 * TGAS;
pub const NO_DEPOSIT: Balance = 0;

#[inline]
pub(crate) fn keccak(input: &[u8]) -> H256 {
    H256::from_slice(&env::keccak256(&input))
}

pub(crate) fn near_account_to_evm_address(addr: &AccountId) -> Address {
    Address::from_slice(&keccak(addr.as_bytes())[12..])
}

pub(crate) fn get_selector(str_selector: &str) -> Vec<u8> {
    keccak(str_selector.as_bytes())[..4].to_vec()
}

pub(crate) fn build_input(str_selector: &str, inputs: &[Token]) -> Vec<u8> {
    let sel = get_selector(str_selector);
    let inputs = ethabi::encode(inputs);
    [sel.as_slice(), inputs.as_slice()].concat().to_vec()
}

pub(crate) fn address_to_string(address: &Address) -> String {
    format!("0x{}", hex::encode(&address.0))
}

pub(crate) fn parse_address(address: &str) -> Address {
    assert!(address.starts_with("0x"), "Address should start with 0x");
    let res = hex::decode(&address[2..]).expect("Failed to parse");
    assert_eq!(res.len(), 20, "Not 20 bytes");
    Address::from_slice(&res)
}

#[ext_contract(ext_aurora_internal)]
trait ExtAuroraInternal {
    #[result_serializer(borsh)]
    fn get_balance(&self, #[serializer(borsh)] address: RawAddress);

    #[result_serializer(borsh)]
    fn get_erc20_from_nep141(&self, #[serializer(borsh)] token_id: AccountId);

    #[result_serializer(borsh)]
    fn view(
        &self,
        #[serializer(borsh)] sender: RawAddress,
        #[serializer(borsh)] address: RawAddress,
        #[serializer(borsh)] amount: RawU256,
        #[serializer(borsh)] input: Vec<u8>,
    );

    #[result_serializer(borsh)]
    fn call(&self, #[serializer(borsh)] call_args: CallArgs);
}

/// Borsh-encoded parameters for the engine `call` function.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct FunctionCallArgsV2 {
    pub contract: RawAddress,
    /// Wei compatible Borsh-encoded value field to attach an ETH balance to the transaction
    pub value: RawU256,
    pub input: Vec<u8>,
}

/// Deserialized values from bytes to current or legacy Borsh-encoded parameters
/// for passing to the engine `call` function, and to provide backward type compatibility
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum CallArgs {
    V2(FunctionCallArgsV2),
    LegacyShit,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    Succeed(Vec<u8>),
    Revert(Vec<u8>),
    OutOfGas,
    OutOfFund,
    OutOfOffset,
    CallTooDeep,
}

pub mod ext_aurora {
    use super::*;

    pub fn get_balance(address: Address) -> Promise {
        ext_aurora_internal::get_balance(address.0, &AURORA.to_string(), NO_DEPOSIT, 5 * TGAS)
    }

    pub fn get_erc20_from_nep141(token_id: &AccountId) -> Promise {
        ext_aurora_internal::get_erc20_from_nep141(
            token_id.clone(),
            &AURORA.to_string(),
            NO_DEPOSIT,
            10 * TGAS,
        )
    }

    fn call(contract: Address, input: Vec<u8>) -> Promise {
        ext_aurora_internal::call(
            CallArgs::V2(FunctionCallArgsV2 {
                contract: contract.0,
                value: RawU256::default(),
                input,
            }),
            &AURORA.to_string(),
            NO_DEPOSIT,
            CALL_GAS,
        )
    }

    pub fn get_erc20_balance(token_address: Address, account_address: Address) -> Promise {
        let input = build_input(
            "balanceOf(address)",
            &[Token::Address(account_address.into())],
        );
        ext_aurora_internal::view(
            account_address.0,
            token_address.0,
            RawU256::default(),
            input,
            &AURORA.to_string(),
            NO_DEPOSIT,
            VIEW_CALL_GAS,
        )
    }

    pub fn erc20_transfer_balance(
        token_address: Address,
        receiver: Address,
        amount: Balance,
    ) -> Promise {
        let input = build_input(
            "transfer(address,uint256)",
            &[
                Token::Address(receiver.into()),
                Token::Uint(U256::from(amount)),
            ],
        );
        call(token_address, input)
    }
}
