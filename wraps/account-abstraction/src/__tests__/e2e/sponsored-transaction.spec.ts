import { PolywrapClient, ClientConfigBuilder } from "@polywrap/client-js";
import * as App from "../types/wrap";
import path from "path";
import { configure } from "../../../client-config";
import { EthersUtils_Module } from "../types/wrap";
import { BigNumber } from "ethers";
import {
  accountAbstractionWrapperUri,
  etherCoreWrapperUri,
  etherUtilsWrapperUri,
  relayerAdapterWrapperUri,
} from "./constants";

jest.setTimeout(60000);

const connection = {
  networkNameOrChainId: "goerli",
};

describe("Sponsored transaction AA wrapper", () => {
  const dirname: string = path.resolve(__dirname);
  const wrapperPath: string = path.join(dirname, "..", "..", "..");
  const accountAbstractionWrapperFsUri = `fs/${wrapperPath}/build`;

  const builder = new ClientConfigBuilder();
  const configuredBuilder = configure(builder);
  configuredBuilder
    .addEnv(accountAbstractionWrapperUri, {
      connection,
    })
    .addEnv(relayerAdapterWrapperUri, {
      relayerApiKey: "AiaCshYRyAUzTNfZZb8LftJaAl2SS3I8YwhJJXc5J7A_",
    })
    .addRedirect(accountAbstractionWrapperUri, accountAbstractionWrapperFsUri);

  const client = new PolywrapClient(configuredBuilder.build());

  it("calls relay transaction", async () => {
    const encodedFunction = await EthersUtils_Module.encodeFunction(
      {
        method: "function store(uint256 num) public",
        args: ["4"],
      },
      client,
      etherUtilsWrapperUri
    );

    if (!encodedFunction.ok) throw encodedFunction.error;
    const metaTransactionData = {
      to: "0x56535D1162011E54aa2F6B003d02Db171c17e41e",
      value: "0",
      data: encodedFunction.value,
      operation: "0",
    };

    const gasLimit = await App.Ethers_Module.estimateTransactionGas(
      {
        tx: {
          to: metaTransactionData.to,
          value: metaTransactionData.value,
          data: metaTransactionData.data,
        },
      },
      client,
      etherCoreWrapperUri
    );
    if (!gasLimit.ok) throw gasLimit.error;

    const gaslimitWithBuffer = BigNumber.from(gasLimit.value)
      .add(250_000)
      .toString();
    const estimation = await App.Relayer_Module.getEstimateFee(
      {
        chainId: 5,
        gasLimit: gaslimitWithBuffer,
      },
      client,
      relayerAdapterWrapperUri
    );
    if (!estimation.ok) throw estimation.error;

    const safeAddress = await App.AccountAbstraction_Module.getSafeAddress(
      {
        config: {
          // saltNonce
        },
      },
      client,
      accountAbstractionWrapperUri
    );
    if (!safeAddress.ok) throw safeAddress.error;
    console.log("Predicted safe address: ", safeAddress.value);
    const safeBalance = await App.Ethers_Module.getBalance(
      {
        address: safeAddress.value,
        connection,
      },
      client,
      etherCoreWrapperUri
    );
    if (!safeBalance.ok) throw safeBalance.error;

    const estimationInEth = await App.EthersUtils_Module.toEth(
      {
        wei: estimation.value,
      },
      client,
      etherUtilsWrapperUri
    );
    if (!estimationInEth.ok) throw estimationInEth.error;
    console.log(`Fee estimation: ${estimationInEth.value} ETH`);

    const metaTransactionOptions = {
      gasLimit: gaslimitWithBuffer,
      isSponsored: true,
    };

    console.log("Relaying sponsored transaction...");
    const result = await App.AccountAbstraction_Module.relayTransaction(
      {
        transaction: metaTransactionData,
        options: metaTransactionOptions,
        config: {
          // saltNonce
        },
      },
      client,
      accountAbstractionWrapperUri
    );
    expect(result.ok).toBeTruthy();
    if (!result.ok) fail(result.error);
    console.log("Transaction has been relayed...");
    console.log(
      `Task URL: https://relay.gelato.digital/tasks/status/${result.value}`
    );
    expect(result.value).toBeTruthy();
  });
});
