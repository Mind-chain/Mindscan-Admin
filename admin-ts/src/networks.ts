import { readFileSync } from "fs";

export const useNetworks = () => {
  const networksPath =
    process.env.NETWORKS_CONFIG_PATH || "./config/networks.json";
  console.log("read networks from path ", networksPath);
  const networksFile = readFileSync(networksPath, "utf-8");
  return JSON.parse(networksFile).networks;
};
