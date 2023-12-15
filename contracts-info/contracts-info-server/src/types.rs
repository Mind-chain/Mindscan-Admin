use std::str::FromStr;

use blockscout_display_bytes::Bytes as DisplayBytes;
use contracts_info_core::TokenInfo;

pub fn token_info_from_proto(proto: crate::proto::TokenInfo) -> Result<TokenInfo, tonic::Status> {
    let token_address = DisplayBytes::from_str(&proto.token_address)
        .map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;
    if token_address.len() != 20 {
        return Err(tonic::Status::invalid_argument(
            "invalid address length, expected 20",
        ));
    };
    Ok(TokenInfo {
        token_address,
        chain_id: proto.chain_id as i64,
        project_name: proto.project_name,
        project_website: proto.project_website,
        project_email: proto.project_email,
        icon_url: proto.icon_url,
        project_description: proto.project_description,
        project_sector: proto.project_sector,
        docs: proto.docs,
        github: proto.github,
        telegram: proto.telegram,
        linkedin: proto.linkedin,
        discord: proto.discord,
        slack: proto.slack,
        twitter: proto.twitter,
        open_sea: proto.open_sea,
        facebook: proto.facebook,
        medium: proto.medium,
        reddit: proto.reddit,
        support: proto.support,
        coin_market_cap_ticker: proto.coin_market_cap_ticker,
        coin_gecko_ticker: proto.coin_gecko_ticker,
        defi_llama_ticker: proto.defi_llama_ticker,
        token_name: proto.token_name,
        token_symbol: proto.token_symbol,
    })
}
