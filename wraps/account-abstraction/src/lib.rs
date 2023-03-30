pub mod wrap;
use polywrap_wasm_rs::wrap_debug_log;
pub use wrap::*;

use account_abstraction::{AccountAbstraction, Safe};

pub mod account_abstraction;

pub fn relay_transaction(args: ArgsRelayTransaction, env: Env) -> String {
    wrap_debug_log("OFFICIALLY GETTING IN RELAY TRANSACTION WASM METHOD");
    if let None = env.connection {
        panic!("No connection given through env")
    }

    wrap_debug_log("before safe new");
    let safe = Safe::new(env.connection.unwrap(), args.config);

    wrap_debug_log("safe instantiated");
    let relayed_tx_hash = safe.relay_transaction(args.transaction, args.options);
    wrap_debug_log("tx relayed");
    relayed_tx_hash
}

pub fn get_safe_address(args: ArgsGetSafeAddress, env: Env) -> String {
    wrap_debug_log("papaya");
    if let None = env.connection {
        panic!("No connection given through env")
    }
    let safe = Safe::new(env.connection.unwrap(), args.config);
    safe.get_address()
}
