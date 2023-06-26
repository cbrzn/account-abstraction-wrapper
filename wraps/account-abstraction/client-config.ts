import { IWrapPackage } from "@polywrap/client-js";
import {
  ethereumProviderPlugin,
  Connection,
  Connections,
} from "@polywrap/ethereum-provider-js";
import { IClientConfigBuilder } from "@polywrap/client-config-builder-js";
import { dateTimePlugin } from "@polywrap/datetime-plugin-js";
import { config } from "dotenv";
import { Wallet } from "ethers";

config();

export function configure(builder: IClientConfigBuilder): IClientConfigBuilder {
  // Jest sets NODE_ENV to "test" when running tests
  const isTestEnv = process.env.NODE_ENV === "test";

  builder.addDefaults();

  if (isTestEnv) {
    if (!process.env.PRIVATE_KEY) {
      throw new Error(
        "You must define a private key in the .env file. See .example.env"
      );
    }

    if (!process.env.RPC_URL) {
      throw new Error(
        "You must define a RPC URL in the .env file. See .example.env"
      );
    }

    builder
      .addPackages({
        "wrap://ens/wraps.eth:ethereum-provider@2.0.0": ethereumProviderPlugin({
          connections: new Connections({
            networks: {
              goerli: new Connection({
                provider: process.env.RPC_URL as string,
                signer: new Wallet(process.env.PRIVATE_KEY as string),
              }),
            },
            defaultNetwork: "goerli",
          }),
        }),
        "wrap://ens/datetime.polywrap.eth": dateTimePlugin({}) as IWrapPackage,
      })
      .addEnv("wrap://wrapper/account-abstraction", {
        connection: {
          networkNameOrChainId: "goerli",
        },
      });
  }

  builder
    .addRedirect(
      "wrap://ens/aa.wraps.eth:relayer-adapter@0.0.1",
      "wrap://fs/../relay/build"
    )
    .addRedirect(
      "wrap://ens/gelato.wraps.eth:relayer@0.0.1",
      "wrap://fs/../../../../polywrap/gelato-relay-polywrap/build"
    );

  return builder;
}
