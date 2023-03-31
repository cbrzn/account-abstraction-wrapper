use polywrap_wasm_rs::{BigInt, wrap_debug_log};

use crate::{
    imported::{
        gelato_relayer_sponsored_call_request::GelatoRelayerSponsoredCallRequest,
        ArgsCallWithSyncFee, ArgsGetEstimatedFee, ArgsSponsoredCall,
        GelatoRelayerCallWithSyncFeeRequest,
    },
    meta_transaction_options::MetaTransactionOptions,
    wrap::GelatoRelayerRelayResponse,
    Env, GelatoRelayerModule, RelayResponse, RelayTransaction, GelatoRelayerRelayRequestOptions,
};

const GELATO_NATIVE_TOKEN_ADDRESS: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";
const GELATO_FEE_COLLECTOR: &str = "0x3AC05161b76a35c1c28dC99Aa01BEd7B24cEA3bf";
const GELATO_RELAY_URL: &str = "https://relay.gelato.digital";
const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
const DEFAULT_RELAYER_API_KEY: &str = "AiaCshYRyAUzTNfZZb8LftJaAl2SS3I8YwhJJXc5J7A_";

pub trait RelayAdapter {
    fn get_fee_collector() -> String;
    fn get_estimate_fee(chain_id: u64, gas_limit: u64, gas_token: Option<String>) -> BigInt;
    fn relay_transaction(transaction: RelayTransaction, env: Option<Env>) -> RelayResponse;
}

pub struct GelatoRelayer;

impl RelayAdapter for GelatoRelayer {
    fn get_estimate_fee(chain_id: u64, gas_limit: u64, gas_token: Option<String>) -> BigInt {
        let fee_token = GelatoRelayer::get_fee_token(gas_token);
        GelatoRelayerModule::get_estimated_fee(&ArgsGetEstimatedFee {
            chain_id: BigInt::from(chain_id),
            gas_limit: BigInt::from(gas_limit),
            payment_token: fee_token,
            is_high_priority: true,
            gas_limit_l1: None,
        })
        .unwrap()
    }

    fn get_fee_collector() -> String {
        GELATO_FEE_COLLECTOR.to_string()
    }

    fn relay_transaction(transaction: RelayTransaction, env: Option<Env>) -> RelayResponse {
        // let mut default_key = DEFAULT_RELAYER_API_KEY;
        let is_sponsored = if let Some(is_sponsored) = transaction.options.is_sponsored {
            if env.is_none() {
                panic!("Sponsor API Key not defined")
                // env.unwrap().relayer_api_key;
                // wrap_debug_log("Sponsor A/PI Key not defined, the default will be used")
            };

            is_sponsored
        } else {
            false
        };

        if is_sponsored {
            GelatoRelayer::sponsor_transaction(
                transaction.target,
                transaction.encoded_transaction,
                transaction.chain_id,
                env.unwrap().relayer_api_key
            )
        } else {
            GelatoRelayer::pay_transaction(
                transaction.target,
                transaction.encoded_transaction,
                transaction.chain_id,
                transaction.options,
            )
        }
    }
}

impl GelatoRelayer {
    pub fn get_fee_token(gas_token: Option<String>) -> String {
        let token = if let Some(gas_token) = gas_token {
            if gas_token != ZERO_ADDRESS {
                Some(gas_token)
            } else {
                None
            }
        } else {
            None
        };

        if token.is_some() {
            token.unwrap()
        } else {
            GELATO_NATIVE_TOKEN_ADDRESS.to_string()
        }
    }

    pub fn sponsor_transaction(
        target: String,
        encoded_transaction: String,
        chain_id: i32,
        sponsor_api_key: String,
    ) -> RelayResponse {
        GelatoRelayerModule::sponsored_call(&ArgsSponsoredCall {
            request: GelatoRelayerSponsoredCallRequest {
                chain_id: chain_id.into(),
                target,
                data: encoded_transaction,
            },
            sponsor_api_key,
            options: None,
        })
        .unwrap()
        .into()
    }

    pub fn pay_transaction(
        target: String,
        encoded_transaction: String,
        chain_id: i32,
        options: MetaTransactionOptions,
    ) -> RelayResponse {
        let fee_token = GelatoRelayer::get_fee_token(options.gas_token);
        GelatoRelayerModule::call_with_sync_fee(&ArgsCallWithSyncFee {
            request: GelatoRelayerCallWithSyncFeeRequest {
                target,
                chain_id: chain_id.into(),
                data: encoded_transaction,
                fee_token,
                is_relay_context: Some(false),
            },
            options: Some(GelatoRelayerRelayRequestOptions {
                gas_limit: Some(options.gas_limit),
                retries: None
            }),
        })
        .unwrap()
        .into()
    }
}

impl From<GelatoRelayerRelayResponse> for RelayResponse {
    fn from(value: GelatoRelayerRelayResponse) -> Self {
        Self {
            task_id: value.task_id,
        }
    }
}
