import { IClientConfigBuilder } from "@polywrap/client-config-builder-js";
import { config } from "dotenv";
import path from "path";

config();

export function configure(builder: IClientConfigBuilder): IClientConfigBuilder {
  // Jest sets NODE_ENV to "test" when running tests
  const isTestEnv = process.env.NODE_ENV === "test";

  const dirname: string = path.resolve(__dirname);
  const localWrapperUri = `fs/${dirname}/build`;

  builder
    .addDefaults()
    .addRedirect(
      "wrap://ens/gelato.wraps.eth:relayer@0.0.1",
      "wrap://fs/../../../../polywrap/gelato-relay-polywrap/build"
    );

  if (isTestEnv) {
    if (!process.env.RELAYER_API_KEY) {
      throw new Error(
        "You must define a relayer api key in the .env file. See .example.env"
      );
    }

    builder
      .addEnv("wrap://ens/gelato.wraps.eth:relayer@0.0.1", {
        relayerApiKey: process.env.RELAYER_API_KEY,
      })
      .addEnv(`wrap://${localWrapperUri}`, {
        relayerApiKey: process.env.RELAYER_API_KEY,
      });
  }

  return builder;
}
