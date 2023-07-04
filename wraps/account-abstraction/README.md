# Safe AA Wrap

The Safe AA Wrap is an implementation of the [Safe{Core} Account Abstraction SDK](https://docs.safe.global/learn/safe-core/safe-core-account-abstraction-sdk) in the form of a Polywrap wrap.

## Requirements

To run the Safe AA wrap you'll need a Polywrap client in your application. See here for installation information: [https://docs.polywrap.io/clients](https://docs.polywrap.io/clients)

## Configuration

The Safe AA Wrap allows you to pass a signer (through browser or with private key) in order to be able to deploy your Safe contract and call its functions.

You will need to add the [ethereum provider plugin](https://github.com/polywrap/ethereum-wallet) to the Polywrap Client's config manually.
You will also need to add the [datetime plugin](https://github.com/polywrap/datetime).

You can configure this using the [Polywrap Client Config Builder](https://docs.polywrap.io/tutorials/use-wraps/configure-client):

```javascript
import {
  ethereumProviderPlugin,
  Connection,
  Connections,
} from "@polywrap/ethereum-provider-js";
import { dateTimePlugin } from "@polywrap/datetime-plugin-js";
import { PolywrapClient, ClientConfigBuilder } from "@polywrap/client-js";

const builder = new ClientConfigBuilder();

builder
  .addDefaults()
  .addPackages({
    // This adds the ethereum provider (wallet) plugin
    "wrap://ens/wraps.eth:ethereum-provider@2.0.0": ethereumProviderPlugin({
      connections: new Connections({
        networks: {
          goerli: new Connection({        
            provider: "RPC_URL", // Add your RPC URL here
            signer: new Wallet("PRIVATE_KEY"), // Add your private key here
            
            // Alternatively, instead of setting provider and signer, you can simply use window.ethereum as your provider:
            // provider: window.ethereum
          }),
        },
        defaultNetwork: "goerli",
      }),
    }),
    // This adds the datetime plugin
    "wrap://ens/datetime.polywrap.eth": dateTimePlugin({}) as IWrapPackage,
  })
  // This URL should be replaced with the Wrap URL of the AA Wrap
  .addEnv("wrap://wrapper/account-abstraction", {
    connection: {
      networkNameOrChainId: "goerli", // Determines which network you're using
    },
  });

// Build the config
const config = builder.build();

// You can now use the Polywrap Client to invoke the AA Wrap
const client = new PolywrapClient(config);

const invocationResult = client.invoke({
  // This URI should be replaced with the Wrap URI of the AA Wrap
  uri: "wrap://wrapper/account-abstraction",
  method: "relayTransaction",
  args: { ... }
})
```

## Run!

With your client successfully configured, you can now run any function on the UniV3 wrap with ease.
See the examples below, or [take a look at the Safe AA wrap tests](https://github.com/cbrzn/account-abstraction-wrapper/tree/main/wraps/account-abstraction/src/__tests__) for invocation examples.

## Examples

### Get Safe Address

Since your Safe address is deterministic based on your wallet's address and a nonce, you can determine it ahead-of-time:

```javascript
const addressResult = await client.invoke({
  // This URI should be replaced with the Wrap URI of the AA Wrap
  uri: "wrap://wrapper/account-abstraction",
  method: "getSafeAddress",
  args: {
    config: {
      saltNonce: "0x258802387238728372837283771" // Replace this with your own salt nonce
    }
  }
});
```

### Relay Sponsored Transaction

Let's assume we have a smart contract with a `store(uint256 num)` function deployed to the address `0x56535D1162011E54aa2F6B003d02Db171c17e41e`.
If we want to, for example, invoke this function, we first need to encode it using our [Ethers Utils Wrap](https://github.com/polywrap/ethers):

```javascript
const encodeResult = await client.invoke({
  uri: "wrap://ens/ethers.wraps.eth:utils@0.1.0",
  method: "encodeFunction",
  args: {
    method: "function store(uint256 num) public",
    args: [
      "14"
    ]
  }
});
```

The Account Abstraction Wrapper's `relayTransaction` method allows you to make a transaction by performing the following steps:

1. It checks whether there's a deployed Safe contract on your predicted address.
2. It deploys a Safe contract to the predicted address if there isn't one.
3. It executes the transaction using the Gelato Relay.

```javascript
await client.invoke({
  // This URI should be replaced with the Wrap URI of the AA Wrap
  uri: "wrap://wrapper/account-abstraction",
  method: "relayTransaction",
  args: {
    transaction: {
      to: "0x56535D1162011E54aa2F6B003d02Db171c17e41e", // Address of the contract you're invoking
      value: "0",
      data: encodeResult.data, // Invocation data.
      operation: "0"
    },
    options: {
      gasLimit: "300000",
      isSponsored: true // Determines whether this should be a sponsored (gasless) transaction or not
    },
    config: {
      // Replace this with your own salt nonce. This is used to deploy your safe contract if there is none.
      // This is not required. If not supplied, the current date/time will be used to generate a salt nonce.
      saltNonce: "0x258802387238728372837283771"
    }
  }
});
```

## Support

For any questions or problems related to the Safe AA wrap or Polywrap at large, please visit our [Discord](https://discord.polywrap.io).
