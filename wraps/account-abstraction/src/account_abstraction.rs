use polywrap_wasm_rs::{BigInt, wrap_debug_log};
use std::collections::BTreeMap;

use crate::{
    imported::{
        ether_core_module::serialization::ArgsGetChainId,
        relayer_meta_transaction_options::RelayerMetaTransactionOptions,
        relayer_module::serialization::ArgsRelayTransaction,
        relayer_relay_transaction::RelayerRelayTransaction,
        safe_contracts_contract_networks_filter::SafeContractsContractNetworksFilter,
        safe_contracts_ethereum_connection,
        safe_contracts_module::serialization::ArgsGetNonce,
        safe_factory_ethereum_connection,
        safe_factory_module::serialization::{ArgsEncodeDeploySafe, ArgsPredictSafeAddress},
        safe_factory_safe_account_config::SafeFactorySafeAccountConfig,
        safe_manager_ethereum_connection,
        safe_manager_module::ArgsGetSignature,
        ArgsEncodeExecTransaction, ArgsEncodeMultiSendData, ArgsGetEstimateFee,
        ArgsGetFeeCollector, ArgsGetSafeContractNetworks, ArgsGetSignerAddress, ArgsSafeIsDeployed,
        SafeFactoryDeploymentInput, SafeFactorySafeDeploymentConfig,
    },
    DeploymentParameters, EtherCoreConnection, EtherCoreModule, MetaTransactionData,
    MetaTransactionOptions, RelayerModule, SafeContractsModule, SafeContractsSafeTransaction,
    SafeContractsSafeTransactionData, SafeContractsSignSignature, SafeFactoryModule,
    SafeManagerModule, SafeManagerSafeTransaction, SafeManagerSafeTransactionData,
    SafeManagerSignSignature, wrap,
};

pub const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

// keccak256(toUtf8Bytes('Safe Account Abstraction'))
pub const PREDETERMINED_SALT_NONCE: &str =
    "0xb1073742015cbcf5a3a4d9d1ae33ecf619439710b89475f92e2abd2117e90f90";

pub trait AccountAbstraction {
    fn relay_transaction(
        &self,
        data: MetaTransactionData,
        options: MetaTransactionOptions,
    ) -> String;
}

#[derive(Clone)]
pub struct Safe {
    signer: String,
    chain_id: i32,
    address: String,
    factory_address: String,
    connection: EtherCoreConnection,
    multi_send_call_only_address: String,
    deployment_info: SafeFactoryDeploymentInput,
}

impl AccountAbstraction for Safe {
    fn relay_transaction(
        &self,
        data: MetaTransactionData,
        options: MetaTransactionOptions,
    ) -> String {

        wrap_debug_log("calling relay transaction :)");
        let standardized_tx = self.standardize_transaction_data(data, options.clone());
        wrap_debug_log("transaction standardized");
        // get signature from signer of the sanitized transaction data
        let manager_connection = safe_manager_ethereum_connection::SafeManagerEthereumConnection {
            node: self.connection.clone().node,
            network_name_or_chain_id: self.connection.clone().network_name_or_chain_id,
        };
        wrap_debug_log("before gettinig signature");
        let transaction = SafeManagerModule::get_signature(&ArgsGetSignature {
            tx: standardized_tx.into(),
            signing_method: None,
            safe_address: self.address.as_str().to_string(),
            connection: manager_connection.clone(),
        })
        .unwrap();

        wrap_debug_log("after getting signature");

        // encode the sanitized transaction w/signature
        let transaction_data =
            SafeContractsModule::encode_exec_transaction(&ArgsEncodeExecTransaction {
                safe_address: self.address.as_str().to_string(),
                safe_transaction: transaction.into(),
            })
            .unwrap();

        let (relay_transaction_target, encoded_transaction) = if self.is_deployed() {
            // if safe is deployed, just execute the previous sanitized transaction
            (self.address.as_str().to_string(), transaction_data)
        } else {
            // if safe is NOT deployed, we need to add the deployment
            let safe_deployment_transaction = MetaTransactionData {
                to: self.factory_address.as_str().to_string(),
                value: BigInt::from(0),
                data: SafeFactoryModule::encode_deploy_safe(&ArgsEncodeDeploySafe {
                    input: self.deployment_info.clone(),
                })
                .unwrap(),
                operation: None,
            };

            let safe_transaction = MetaTransactionData {
                to: self.address.as_str().to_string(),
                value: BigInt::from(0),
                data: transaction_data,
                operation: None,
            };

            let multi_send_data =
                SafeManagerModule::encode_multi_send_data(&ArgsEncodeMultiSendData {
                    txs: vec![safe_deployment_transaction.into(), safe_transaction.into()],
                })
                .unwrap();

            (
                self.multi_send_call_only_address.as_str().to_string(),
                multi_send_data,
            )
        };

        let relay_transaction = RelayerRelayTransaction {
            chain_id: self.chain_id,
            encoded_transaction,
            target: relay_transaction_target,
            options: RelayerMetaTransactionOptions {
                gas_limit: options.gas_limit,
                gas_token: options.gas_token,
                is_sponsored: options.is_sponsored,
            },
        };
        let transaction_relayed = RelayerModule::relay_transaction(&ArgsRelayTransaction {
            transaction: relay_transaction,
        })
        .unwrap();

        transaction_relayed.task_id
    }
}

impl Safe {
    pub fn new(connection: EtherCoreConnection, params: Option<DeploymentParameters>) -> Self {
        let factory_connection = safe_factory_ethereum_connection::SafeFactoryEthereumConnection {
            node: connection.clone().node,
            network_name_or_chain_id: connection.clone().network_name_or_chain_id,
        };

        wrap_debug_log("before chain id");
        let chain_id = EtherCoreModule::get_chain_id(&ArgsGetChainId {
            connection: Some(connection.clone()),
        })
        .unwrap();

        wrap_debug_log("chain id fetched");
        let signer = EtherCoreModule::get_signer_address(&ArgsGetSignerAddress {
            connection: Some(connection.clone()),
        })
        .unwrap();
    wrap_debug_log("signer fetched");

        let mut custom_contract_addresses = None;
        let mut factory_address = String::new();
        let mut salt_nonce = PREDETERMINED_SALT_NONCE.to_string();

        if let Some(config) = params {
            if let Some(salt) = config.salt_nonce {
                salt_nonce = salt
            }

            if let Some(custom_contracts) = config.custom_contract_addresses {
                if let Some(factory_contract) = &custom_contracts.proxy_factory_contract {
                    factory_address = factory_contract.as_str().to_string();
                }
                custom_contract_addresses = Some(custom_contracts)
            }
        }

        let get_safe_contract_network_args = ArgsGetSafeContractNetworks {
            chain_id: chain_id.as_str().to_string(),
            is_l1_safe: None,
            version: String::from("1.3.0"),
            filter: Some(SafeContractsContractNetworksFilter {
                fallback_handler_address: false,
                multi_send_address: false,
                multi_send_call_only_address: true,
                safe_master_copy_address: false,
                safe_proxy_factory_address: factory_address.is_empty(),
            }),
        };


        wrap_debug_log("before getting safe contract");
        let safe_contracts_networks =
            SafeContractsModule::get_safe_contract_networks(&get_safe_contract_network_args)
                .unwrap();


                wrap_debug_log("after getting safe contract");

        if factory_address.is_empty() {
            if let Some(address) = safe_contracts_networks.safe_proxy_factory_address {
                factory_address = address;
            }
        };

        let multi_send_call_only_address =
            if let Some(address) = safe_contracts_networks.multi_send_call_only_address {
                address
            } else {
                panic!("Could not fetch multisend contract")
            };

        let deployment_config = SafeFactorySafeDeploymentConfig {
            salt_nonce,
            is_l1_safe: None,
            version: None,
        };
        let deployment_info = SafeFactoryDeploymentInput {
            safe_account_config: SafeFactorySafeAccountConfig {
                owners: vec![signer.as_str().to_string()],
                threshold: 1,
                fallback_handler: None,
                payment: None,
                payment_receiver: None,
                data: None,
                payment_token: None,
                to: None,
            },
            safe_deployment_config: Some(deployment_config),
            custom_contract_addresses,
            connection: Some(factory_connection),
        };
        let predict_address_args = ArgsPredictSafeAddress {
            input: deployment_info.clone(),
        };

        wrap_debug_log("before predicting safe address");
        let address = SafeFactoryModule::predict_safe_address(&predict_address_args).unwrap();

        wrap_debug_log("after predicting safe address");
        Safe {
            chain_id: chain_id.parse::<i32>().unwrap(),
            signer: signer.as_str().to_string(),
            address,
            factory_address,
            multi_send_call_only_address,
            connection,
            deployment_info,
        }
    }

    pub fn get_address(self) -> String {
        self.address
    }

    pub fn is_deployed(&self) -> bool {
        //@TODO: Check why this does not works
        // SafeFactoryModule::safe_is_deployed(&ArgsSafeIsDeployed {
        //     safe_address: self.address.as_str().to_string(),
        //     connection: safe_factory_ethereum_connection::SafeFactoryEthereumConnection {
        //         node: self.connection.clone().node,
        //         network_name_or_chain_id: self.connection.clone().network_name_or_chain_id,
        //     },
        // })
        // .unwrap()
        self.get_nonce() > BigInt::from(0)
    }

    pub fn get_nonce(&self) -> BigInt {
        let connection = safe_contracts_ethereum_connection::SafeContractsEthereumConnection {
            node: self.connection.clone().node,
            network_name_or_chain_id: self.connection.clone().network_name_or_chain_id,
        };

        let nonce = SafeContractsModule::get_nonce(&ArgsGetNonce {
            address: self.address.as_str().to_string(),
            connection: Some(connection),
        });

        if let Ok(n) = nonce {
            n
        } else {
            BigInt::from(0)
        }
    }

    pub fn get_signer_address(&self) -> &String {
        &self.signer
    }

    fn standardize_transaction_data(
        &self,
        transaction: MetaTransactionData,
        options: MetaTransactionOptions,
    ) -> SafeContractsSafeTransaction {
        let is_sponsored = if let Some(i) = options.is_sponsored {
            i
        } else {
            false
        };

        let gas_token = if let Some(token) = options.gas_token {
            Box::new(token)
        } else {
            Box::new(ZERO_ADDRESS.to_string())
        };

        let estimation = RelayerModule::get_estimate_fee(&ArgsGetEstimateFee {
            chain_id: self.chain_id,
            gas_limit: options.gas_limit,
            gas_token: Some(*gas_token.clone()),
        })
        .unwrap();

        let base_gas = if is_sponsored {
            BigInt::from(0)
        } else {
            estimation
        };

        let gas_price = if is_sponsored {
            BigInt::from(0)
        } else {
            BigInt::from(1)
        };

        let refund_receiver = if is_sponsored {
            ZERO_ADDRESS.to_string()
        } else {
            RelayerModule::get_fee_collector(&ArgsGetFeeCollector {}).unwrap()
        };

        SafeContractsSafeTransaction {
            data: SafeContractsSafeTransactionData {
                data: transaction.data,
                value: transaction.value,
                to: transaction.to,
                operation: transaction.operation,
                safe_tx_gas: Some(BigInt::from(0)),
                base_gas: Some(base_gas),
                gas_price: Some(gas_price),
                gas_token: Some(*gas_token),
                refund_receiver: Some(refund_receiver),
                nonce: Some(self.get_nonce()),
            },
            signatures: None,
        }
    }
}

impl From<MetaTransactionData> for SafeManagerSafeTransactionData {
    fn from(meta_tx: MetaTransactionData) -> Self {
        SafeManagerSafeTransactionData {
            data: meta_tx.data,
            to: meta_tx.to,
            value: meta_tx.value,
            operation: meta_tx.operation,
            safe_tx_gas: None,
            base_gas: None,
            gas_price: None,
            gas_token: None,
            refund_receiver: None,
            nonce: None,
        }
    }
}

impl From<SafeManagerSafeTransaction> for SafeContractsSafeTransaction {
    fn from(manager_tx: SafeManagerSafeTransaction) -> Self {
        let transaction_data = SafeContractsSafeTransactionData {
            data: manager_tx.data.data,
            to: manager_tx.data.to,
            value: manager_tx.data.value,
            operation: manager_tx.data.operation,
            safe_tx_gas: manager_tx.data.safe_tx_gas,
            base_gas: manager_tx.data.base_gas,
            gas_price: manager_tx.data.gas_price,
            gas_token: manager_tx.data.gas_token,
            refund_receiver: manager_tx.data.refund_receiver,
            nonce: manager_tx.data.nonce,
        };

        let transaction_signatures = if let Some(s) = manager_tx.signatures {
            let mut signatures = BTreeMap::new();
            for (address, signature) in s.into_iter() {
                signatures.insert(
                    address,
                    SafeContractsSignSignature {
                        data: signature.data,
                        signer: signature.signer,
                    },
                );
            }
            Some(signatures)
        } else {
            None
        };

        SafeContractsSafeTransaction {
            data: transaction_data,
            signatures: transaction_signatures,
        }
    }
}

impl From<SafeContractsSafeTransaction> for SafeManagerSafeTransaction {
    fn from(safe_tx: SafeContractsSafeTransaction) -> Self {
        let transaction_data = SafeManagerSafeTransactionData {
            data: safe_tx.data.data,
            to: safe_tx.data.to,
            value: safe_tx.data.value,
            operation: safe_tx.data.operation,
            safe_tx_gas: safe_tx.data.safe_tx_gas,
            base_gas: safe_tx.data.base_gas,
            gas_price: safe_tx.data.gas_price,
            gas_token: safe_tx.data.gas_token,
            refund_receiver: safe_tx.data.refund_receiver,
            nonce: safe_tx.data.nonce,
        };

        let transaction_signatures = if let Some(s) = safe_tx.signatures {
            let mut signatures = BTreeMap::new();
            for (address, signature) in s.into_iter() {
                signatures.insert(
                    address,
                    SafeManagerSignSignature {
                        data: signature.data,
                        signer: signature.signer,
                    },
                );
            }
            Some(signatures)
        } else {
            None
        };

        SafeManagerSafeTransaction {
            data: transaction_data,
            signatures: transaction_signatures,
        }
    }
}
