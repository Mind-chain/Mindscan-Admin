use contracts_info_proto::blockscout::contracts_info::v1 as contracts_info_v1;
use serde::Serialize;
use std::str::FromStr;

#[derive(Clone, Debug, Default, PartialOrd, PartialEq, Eq, Hash, Serialize)]
pub struct TokenInfo {
    pub token_address: ethers::types::Address,
    pub project_name: Option<String>,
    pub project_website: Option<String>,
    pub project_email: Option<String>,
    pub icon_url: Option<String>,
    pub project_description: Option<String>,
    pub project_sector: Option<String>,
    pub docs: Option<String>,
    pub github: Option<String>,
    pub telegram: Option<String>,
    pub linkedin: Option<String>,
    pub discord: Option<String>,
    pub slack: Option<String>,
    pub twitter: Option<String>,
    pub open_sea: Option<String>,
    pub facebook: Option<String>,
    pub medium: Option<String>,
    pub reddit: Option<String>,
    pub support: Option<String>,
    pub coin_market_cap_ticker: Option<String>,
    pub coin_gecko_ticker: Option<String>,
    pub defi_llama_ticker: Option<String>,
    // pub token_name: Option<String>,
    // pub token_symbol: Option<String>,
}

impl TokenInfo {
    pub fn default_with_address(token_address: ethers::types::Address) -> TokenInfo {
        TokenInfo {
            token_address,
            ..Default::default()
        }
    }

    pub fn into_proto(self, chain_id: u64) -> contracts_info_v1::TokenInfo {
        contracts_info_v1::TokenInfo {
            token_address: format!("{:x}", self.token_address),
            chain_id,
            project_name: self.project_name,
            project_website: self.project_website.unwrap_or_default(),
            project_email: self.project_email.unwrap_or_default(),
            icon_url: self.icon_url.unwrap_or_default(),
            project_description: self.project_description.unwrap_or_default(),
            project_sector: self.project_sector,
            docs: self.docs,
            github: self.github,
            telegram: self.telegram,
            linkedin: self.linkedin,
            discord: self.discord,
            slack: self.slack,
            twitter: self.twitter,
            open_sea: self.open_sea,
            facebook: self.facebook,
            medium: self.medium,
            reddit: self.reddit,
            support: self.support,
            coin_market_cap_ticker: self.coin_market_cap_ticker,
            coin_gecko_ticker: self.coin_gecko_ticker,
            defi_llama_ticker: self.defi_llama_ticker,
            // token_name: self.token_name,
            // token_symbol: self.token_symbol,
            token_name: None,
            token_symbol: None,
        }
    }

    pub fn merge(&mut self, other: Self) {
        self.project_name = self.project_name.take().or(other.project_name);
        self.project_website = self.project_website.take().or(other.project_website);
        self.project_email = self.project_email.take().or(other.project_email);
        self.icon_url = self.icon_url.take().or(other.icon_url);
        self.project_description = self
            .project_description
            .take()
            .or(other.project_description);
        self.project_sector = self.project_sector.take().or(other.project_sector);
        self.docs = self.docs.take().or(other.docs);
        self.github = self.github.take().or(other.github);
        self.telegram = self.telegram.take().or(other.telegram);
        self.linkedin = self.linkedin.take().or(other.linkedin);
        self.discord = self.discord.take().or(other.discord);
        self.slack = self.slack.take().or(other.slack);
        self.twitter = self.twitter.take().or(other.twitter);
        self.open_sea = self.open_sea.take().or(other.open_sea);
        self.facebook = self.facebook.take().or(other.facebook);
        self.medium = self.medium.take().or(other.medium);
        self.reddit = self.reddit.take().or(other.reddit);
        self.support = self.support.take().or(other.support);
        self.coin_market_cap_ticker = self
            .coin_market_cap_ticker
            .take()
            .or(other.coin_market_cap_ticker);
        self.coin_gecko_ticker = self.coin_gecko_ticker.take().or(other.coin_gecko_ticker);
        self.defi_llama_ticker = self.defi_llama_ticker.take().or(other.defi_llama_ticker);
    }

    pub fn validate(mut self, extractor: &'static str) -> Self {
        macro_rules! check_field {
            ($field:ident, $predicate:expr, $msg:expr) => {
                if let Some(value) = &self.$field {
                    if !$predicate(value) {
                        self.$field = None;
                        tracing::info!(
                            "{} - {extractor}: `{}` {}",
                            ethers::utils::to_checksum(&self.token_address, None),
                            stringify!($field),
                            $msg
                        )
                    }
                }
            };
        }

        macro_rules! check_has_alphabetic_characters {
            ($field:ident) => {
                let predicate = |value: &str| value.contains(|c: char| c.is_alphabetic());
                check_field!($field, predicate, "has no alphabetic characters")
            };
        }

        macro_rules! check_is_valid_url {
            ($field:ident) => {
                let predicate = |value: &str| url::Url::from_str(value).is_ok();
                check_field!($field, predicate, "is not a valid url")
            };
        }

        check_has_alphabetic_characters!(project_name);
        check_is_valid_url!(project_website);
        check_has_alphabetic_characters!(project_email);
        check_is_valid_url!(icon_url);
        check_has_alphabetic_characters!(project_description);
        check_has_alphabetic_characters!(project_sector);
        check_is_valid_url!(docs);
        check_is_valid_url!(github);
        check_is_valid_url!(telegram);
        check_is_valid_url!(linkedin);
        check_is_valid_url!(discord);
        check_is_valid_url!(slack);
        check_is_valid_url!(twitter);
        check_is_valid_url!(open_sea);
        check_is_valid_url!(facebook);
        check_is_valid_url!(medium);
        check_is_valid_url!(reddit);
        check_is_valid_url!(support);
        check_is_valid_url!(coin_market_cap_ticker);
        check_is_valid_url!(coin_gecko_ticker);
        check_is_valid_url!(defi_llama_ticker);

        self
    }
}

#[cfg(test)]
mod tests {
    use crate::TokenInfo;
    use ethers::types::Address;

    #[test]
    pub fn merge_token_infos() {
        let token_address = Address::from([1u8; 20]);

        let expected = TokenInfo {
            token_address,
            project_name: Some("project_name".to_string()),
            project_website: Some("project_website".to_string()),
            project_email: Some("project_email".to_string()),
            icon_url: Some("icon_url".to_string()),
            project_description: Some("project_description".to_string()),
            project_sector: Some("project_sector".to_string()),
            docs: Some("docs".to_string()),
            github: Some("github".to_string()),
            telegram: Some("telegram".to_string()),
            linkedin: Some("linkedin".to_string()),
            discord: Some("discord".to_string()),
            slack: Some("slack".to_string()),
            twitter: Some("twitter".to_string()),
            open_sea: Some("open_sea".to_string()),
            facebook: Some("facebook".to_string()),
            medium: Some("medium".to_string()),
            reddit: Some("reddit".to_string()),
            support: Some("support".to_string()),
            coin_market_cap_ticker: Some("coin_market_cap_ticker".to_string()),
            coin_gecko_ticker: Some("coin_gecko_ticker".to_string()),
            defi_llama_ticker: Some("defi_llama_ticker".to_string()),
        };

        let mut token_info_1 = TokenInfo {
            token_address,
            project_name: expected.project_name.clone(),
            project_website: expected.project_website.clone(),
            project_email: expected.project_email.clone(),
            icon_url: expected.icon_url.clone(),
            project_description: expected.project_description.clone(),
            project_sector: expected.project_sector.clone(),
            docs: expected.docs.clone(),
            ..Default::default()
        };

        let token_info_2 = TokenInfo {
            token_address,
            github: expected.github.clone(),
            telegram: expected.telegram.clone(),
            linkedin: expected.linkedin.clone(),
            discord: expected.discord.clone(),
            slack: expected.slack.clone(),
            twitter: expected.twitter.clone(),
            open_sea: expected.open_sea.clone(),
            facebook: expected.facebook.clone(),
            medium: expected.medium.clone(),
            reddit: expected.reddit.clone(),
            support: expected.support.clone(),
            coin_market_cap_ticker: expected.coin_market_cap_ticker.clone(),
            coin_gecko_ticker: expected.coin_gecko_ticker.clone(),
            defi_llama_ticker: expected.defi_llama_ticker.clone(),
            ..Default::default()
        };

        token_info_1.merge(token_info_2);
        assert_eq!(expected, token_info_1, "Invalid merge result");
    }
}
