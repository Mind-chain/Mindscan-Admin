use crate::{settings::TrustWalletExtractorSettings, Extractor};
use anyhow::Context;
use ethers::addressbook::Address;
use git2::Repository;
use serde::Deserialize;
use std::{collections::HashSet, fs::File, io::BufReader, path::PathBuf, str::FromStr};

const ID: &str = "trust_wallet";

const REPO_URL: &str = "https://github.com/trustwallet/assets.git";

const TOKEN_LOGO_URL_TEMPLATE: &str = "https://raw.githubusercontent.com/trustwallet/assets/master/blockchains/{CHAIN_NAME}/assets/{TOKEN_ADDRESS}/logo.png";

const ASSETS_DIR_TEMPLATE: &str = "blockchains/{CHAIN_NAME}/assets";

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Link {
    name: String,
    url: String,
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
struct Links {
    links: Vec<Link>,
}

impl Links {
    pub fn get(&self, name: &str) -> Option<&String> {
        self.links
            .iter()
            .find_map(|link| (link.name == name).then_some(&link.url))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Status {
    Active,
    Spam,
    Abandoned,
    Other(String),
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct TokenInfo {
    name: String,
    website: Option<String>,
    description: Option<String>,
    links: Option<Links>,
    status: Status,
}

impl From<(Address, TokenInfo, String)> for crate::TokenInfo {
    fn from((token_address, token, icon_url): (Address, TokenInfo, String)) -> Self {
        let links = token.links.unwrap_or_default();
        Self {
            token_address,
            project_name: Some(token.name),
            project_website: token.website,
            project_email: None,
            icon_url: Some(icon_url),
            project_description: token.description,
            project_sector: None,
            docs: links.get("docs").cloned(),
            github: links.get("github").cloned(),
            telegram: links.get("telegram").cloned(),
            linkedin: links.get("linkedin").cloned(),
            discord: links.get("discord").cloned(),
            slack: links.get("slack").cloned(),
            twitter: links.get("twitter").cloned(),
            open_sea: links.get("opensea").cloned(),
            facebook: links.get("facebook").cloned(),
            medium: links.get("medium").cloned(),
            reddit: links.get("reddit").cloned(),
            coin_market_cap_ticker: links.get("coinmarketcap").cloned(),
            coin_gecko_ticker: links.get("coingecko").cloned(),
            defi_llama_ticker: None,
            support: None,
        }
        .validate(ID)
    }
}

impl TokenInfo {
    fn validate(mut self, chain_name: &str, token_address: &Address) -> Option<Self> {
        if self.status != Status::Active {
            tracing::debug!(
                "{} - {chain_name}: token info has '{:?}' status; skipping",
                ethers::utils::to_checksum(token_address, None),
                self.status
            );
            return None;
        }

        // Decided not to return tokens without a proper name
        if !self.name.contains(|c: char| c.is_alphanumeric()) {
            return None;
        }

        // To decrease the amount of warnings from the final TokenInfo validation function
        self.website = self
            .website
            .filter(|value| !value.is_empty() && value != "-");
        self.website = self
            .website
            .map(|website| match url::Url::from_str(&website) {
                // To parse websites of the following form: "app.ichi.org"
                // (https://github.com/trustwallet/assets/blob/master/blockchains/ethereum/assets/0xcA37530E7c5968627BE470081d1C993eb1dEaf90/info.json)
                Err(url::ParseError::RelativeUrlWithoutBase) => {
                    format!("https://{website}")
                }
                _ => website,
            });

        self.description = self
            .description
            .filter(|value| !value.is_empty() && value != "-");

        Some(self)
    }
}

pub struct TrustWalletExtractor {
    repo_dir: PathBuf,
    validate_token_icon_url: bool,
}

impl TrustWalletExtractor {
    pub async fn init(
        settings: TrustWalletExtractorSettings,
    ) -> Result<TrustWalletExtractor, anyhow::Error> {
        let repo_dir = if let Some(dir) = settings.repo_dir {
            if !dir.is_dir() {
                return Err(anyhow::anyhow!("target repo directory does not exist"));
            };
            dir
        } else {
            let tempdir = tempdir::TempDir::new("trust-wallet-assets")
                .context("creation of temporary directory")?;
            let _repo = Repository::clone(REPO_URL, tempdir.path()).context(format!(
                "cloning repository from {} to {}",
                REPO_URL,
                tempdir.path().to_string_lossy()
            ))?;
            tempdir.into_path()
        };

        Ok(Self {
            repo_dir,
            validate_token_icon_url: settings.validate_token_icon_url,
        })
    }

    fn construct_token_icon_url(chain_name: &str, token_address: Address) -> String {
        let token_address = ethers::utils::to_checksum(&token_address, None);
        TOKEN_LOGO_URL_TEMPLATE
            .replace("{CHAIN_NAME}", chain_name)
            .replace("{TOKEN_ADDRESS}", &token_address)
    }

    async fn validate_token_icon_url(url: &str) -> Result<(), anyhow::Error> {
        let response = reqwest::get(url).await.context("sending request")?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "request returned with non-success status: {}",
                response.status()
            ));
        }
        Ok(())
    }

    fn get_chain_name(&self, chain_id: u64, default: &str) -> String {
        match chain_id {
            1 => "ethereum",
            10 => "optimism",
            420 => "optimismgoerli",
            100 => "xdai",
            _ => default,
        }
        .to_string()
    }
}

#[async_trait::async_trait]
impl Extractor for TrustWalletExtractor {
    type Error = anyhow::Error;

    async fn token_list(
        &self,
        chain_id: u64,
        default_chain_name: &str,
    ) -> Result<HashSet<Address>, Self::Error> {
        let chain_name = self.get_chain_name(chain_id, default_chain_name);
        let dir = {
            let mut dir = self.repo_dir.clone();
            let assets_dir = ASSETS_DIR_TEMPLATE.replace("{CHAIN_NAME}", &chain_name);
            dir.push(assets_dir);
            dir
        };

        if !dir.is_dir() {
            tracing::warn!("directory does not exist: {dir:?}");
            return Ok(HashSet::new());
        }

        let mut token_list = HashSet::new();
        for entry in dir
            .read_dir()
            .context(format!("reading directory: {dir:?}"))?
        {
            let entry = entry.context(format!("reading directory entry: {dir:?}"))?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(os_str) = path.file_name() {
                    if let Some(name) = os_str.to_str() {
                        if let Ok(address) = Address::from_str(name) {
                            token_list.insert(address);
                        }
                    }
                }
            }
        }

        Ok(token_list)
    }

    async fn token_info(
        &self,
        chain_id: u64,
        default_chain_name: &str,
        token_address: Address,
    ) -> Result<Option<crate::TokenInfo>, Self::Error> {
        let chain_name = self.get_chain_name(chain_id, default_chain_name);

        let token_info_path = {
            let mut dir = self.repo_dir.clone();
            let assets_dir = ASSETS_DIR_TEMPLATE.replace("{CHAIN_NAME}", &chain_name);
            dir.push(format!(
                "{assets_dir}/{}",
                ethers::utils::to_checksum(&token_address, None)
            ));
            dir.push("info.json");
            dir
        };
        let token_info_file = File::open(token_info_path).context("reading info.json")?;

        let token_info = {
            let token_info: TokenInfo = serde_json::from_reader(BufReader::new(token_info_file))
                .context("converting info.json content to TokenInfo")?;

            match token_info.validate(default_chain_name, &token_address) {
                None => return Ok(None),
                Some(token_info) => token_info,
            }
        };

        let logo_url =
            TrustWalletExtractor::construct_token_icon_url(default_chain_name, token_address);
        if self.validate_token_icon_url {
            TrustWalletExtractor::validate_token_icon_url(&logo_url)
                .await
                .context(format!("token icon url validation: {logo_url}"))?
        }

        Ok(Some(crate::TokenInfo::from((
            token_address,
            token_info,
            logo_url,
        ))))
    }
}

mod status {
    use super::Status;
    use serde::{de, de::Visitor, Deserialize, Deserializer};
    use std::fmt;

    struct StatusVisitor;

    impl<'de> Visitor<'de> for StatusVisitor {
        type Value = Status;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Status, E>
        where
            E: de::Error,
        {
            match value {
                "active" => Ok(Status::Active),
                "spam" => Ok(Status::Spam),
                "abandoned" => Ok(Status::Abandoned),
                _ => Ok(Status::Other(value.to_owned())),
            }
        }
    }

    impl<'de> Deserialize<'de> for Status {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_string(StatusVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn token_info_status_deserialization() {
        let statuses = [
            ("active", Status::Active),
            ("spam", Status::Spam),
            ("abandoned", Status::Abandoned),
            ("other", Status::Other("other".into())),
        ];

        for (status, expected) in statuses {
            let json = serde_json::json!({
                "name": "Trust Wallet Token",
                "website": "https://trustwallet.com",
                "description": "Utility token to increase adoption of cryptocurrency.",
                "status": status
            });

            let token_info: TokenInfo =
                serde_json::from_value(json).expect("Token info deserialization failed");
            assert_eq!(expected, token_info.status, "Invalid token info status");
        }
    }
}
