import { AppError, BaseRecord } from "adminjs";
import { makePostRequest } from "./request";

export const sendRecordToService = async (record: BaseRecord) => {
  const tokenInfo = {
    tokenAddress: record.get("token_address") ?? record.get("address"),
    chainId: String(record.get("chain_id")),

    projectName: record.get("project_name"),
    projectWebsite: record.get("project_website"),
    projectEmail: record.get("project_email"),
    iconUrl: record.get("icon_url"),
    projectSector: record.get("project_sector"),
    projectDescription: record.get("project_description"),

    docs: record.get("docs"),
    github: record.get("github"),
    telegram: record.get("telegram"),
    linkedin: record.get("linkedin"),
    discord: record.get("discord"),
    slack: record.get("slack"),
    twitter: record.get("twitter"),
    open_sea: record.get("open_sea"),
    facebook: record.get("facebook"),
    medium: record.get("medium"),
    reddit: record.get("reddit"),
    support: record.get("support"),

    coinMarketCapTicker: record.get("coin_market_cap_ticker"),
    coinGeckoTicker: record.get("coin_gecko_ticker"),
    defiLlamaTicker: record.get("defi_llama_ticker"),
    tokenName: record.get("token_name"),
    tokenSymbol: record.get("token_symbol"),
  };
  return await importTokenInfo(tokenInfo);
};

const importTokenInfo = async (tokenInfo: any) => {
  console.log(
    `import token info. address: ${tokenInfo.tokenAddress}, chainId: ${tokenInfo.chainId}`
  );
  const CONTRACTS_INFO_HOST = process.env.CONTRACTS_INFO_HOST;
  const CONTRACTS_INFO_API_KEY = process.env.CONTRACTS_INFO_API_KEY;
  const url = `${CONTRACTS_INFO_HOST}/api/v1/admin/token-infos:import`;
  const data = {
    tokenInfo: tokenInfo,
  };
  const headers = {
    "x-api-key": CONTRACTS_INFO_API_KEY,
  };
  const response = await makePostRequest(url, data, headers);
  if (response.status === 200) {
    return response;
  } else {
    const json = await response.json();
    let text = "";
    if (json.message) {
      text = json.message;
    } else {
      text = await response.text();
    }
    if (response.status == 401) {
      console.error("invalid api_key for contracts_info");
      throw new AppError(
        "failed to import token info: unauthorized, check api key",
        { error: text, status: response.status }
      );
    }
    throw new AppError(text, { status: response.status });
  }
};
