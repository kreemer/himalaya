use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use quick_xml::de as xml;
use reqwest::Client;
use reqwest::Method;
use serde::Deserialize;

use crate::domain::{Card, CardRepository};

pub struct RemoteCardRepository;

impl CardRepository for RemoteCardRepository {
    fn create(_card: Card) -> Result<()> {
        todo!()
    }

    fn read(_id: String) -> Result<Card> {
        todo!()
    }

    fn read_all() -> Result<Vec<Card>> {
        todo!()
    }

    fn update(_card: Card) -> Result<()> {
        todo!()
    }

    fn delete(_id: String) -> Result<()> {
        todo!()
    }
}

// Common structs

#[derive(Debug, Deserialize)]
pub struct Multistatus<T> {
    #[serde(rename = "response")]
    pub responses: Vec<Response<T>>,
}

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    pub href: Href,
    pub propstat: Propstat<T>,
}

#[derive(Debug, Deserialize)]
pub struct Propstat<T> {
    pub prop: T,
    pub status: Option<Status>,
}

#[derive(Debug, Deserialize)]
pub struct Href {
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Status {
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Ctag {
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Etag {
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct LastModified {
    #[serde(with = "date_parser", rename = "$value")]
    pub value: DateTime<Utc>,
}

mod date_parser {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc2822(&s)
            .map(|d| d.into())
            .map_err(serde::de::Error::custom)
    }
}

// Current user principal structs

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct CurrentUserPrincipalProp {
    pub current_user_principal: CurrentUserPrincipal,
}

#[derive(Debug, Deserialize)]
struct CurrentUserPrincipal {
    pub href: Href,
}

// Addressbook home set structs

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct AddressbookHomeSetProp {
    pub addressbook_home_set: AddressbookHomeSet,
}

#[derive(Debug, Deserialize)]
struct AddressbookHomeSet {
    pub href: Href,
}

// Addressbook structs

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct AddressbookProp {
    pub resourcetype: AddressbookResourceType,
}

#[derive(Debug, Deserialize)]
struct AddressbookResourceType {
    pub addressbook: Option<Addressbook>,
}

#[derive(Debug, Deserialize)]
struct Addressbook {}

// Address data structs

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddressDataProp {
    pub address_data: AddressData,
    pub getetag: Etag,
    pub getlastmodified: LastModified,
}

#[derive(Debug, Deserialize)]
pub struct AddressData {
    #[serde(rename = "$value")]
    pub value: String,
}

// Ctag structs

#[derive(Debug, Deserialize)]
pub struct CtagProp {
    pub getctag: Ctag,
}

// Methods

fn propfind() -> Result<Method> {
    Method::from_bytes(b"PROPFIND").context(r#"cannot create custom method "PROPFIND""#)
}

fn report() -> Result<Method> {
    Method::from_bytes(b"REPORT").context(r#"cannot create custom method "REPORT""#)
}

pub async fn fetch_current_user_principal_url(
    host: &str,
    path: String,
    client: &Client,
) -> Result<String> {
    let res = client
        .request(propfind()?, format!("{}{}", host, path))
        .basic_auth("user", Some(""))
        .body(
            r#"
            <D:propfind xmlns:D="DAV:">
                <D:prop>
                    <D:current-user-principal />
                </D:prop>
            </D:propfind>
            "#,
        )
        .send()
        .await
        .context("cannot send current user principal request")?;
    let res = res
        .text()
        .await
        .context("cannot extract text body from current user principal response")?;
    let res: Multistatus<CurrentUserPrincipalProp> =
        xml::from_str(&res).context("cannot parse current user principal response")?;

    Ok(res
        .responses
        .first()
        .map(|res| {
            res.propstat
                .prop
                .current_user_principal
                .href
                .value
                .to_owned()
        })
        .unwrap_or(path))
}

pub async fn fetch_addressbook_home_set_url(
    host: &str,
    path: String,
    client: &Client,
) -> Result<String> {
    let res = client
        .request(propfind()?, format!("{}{}", host, path))
        .basic_auth("user", Some(""))
        .body(
            r#"
            <D:propfind xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:carddav">
                <D:prop>
                    <C:addressbook-home-set />
                </D:prop>
            </D:propfind>
            "#,
        )
        .send()
        .await
        .context("cannot send addressbook home set request")?;
    let res = res
        .text()
        .await
        .context("cannot extract text body from addressbook home set response")?;
    let res: Multistatus<AddressbookHomeSetProp> =
        xml::from_str(&res).context("cannot parse addressbook home set response")?;

    Ok(res
        .responses
        .first()
        .map(|res| res.propstat.prop.addressbook_home_set.href.value.to_owned())
        .unwrap_or(path))
}

pub async fn fetch_addressbook_url(host: &str, path: String, client: &Client) -> Result<String> {
    let res = client
        .request(propfind()?, host)
        .basic_auth("user", Some(""))
        .send()
        .await
        .context("cannot send addressbook request")?;
    let res = res
        .text()
        .await
        .context("cannot extract text body from addressbook response")?;
    let res: Multistatus<AddressbookProp> =
        xml::from_str(&res).context("cannot parse addressbook response")?;

    Ok(res
        .responses
        .iter()
        .find(|res| {
            let valid_status = res
                .propstat
                .status
                .as_ref()
                .map(|s| s.value.ends_with("200 OK"))
                .unwrap_or(false);
            let has_addressbook = res
                .propstat
                .prop
                .resourcetype
                .addressbook
                .as_ref()
                .is_some();

            valid_status && has_addressbook
        })
        .map(|res| res.href.value.to_owned())
        .unwrap_or(path))
}
