use imap;
use native_tls::{self, TlsConnector, TlsStream};
use std::{fmt, net::TcpStream, result};

use crate::config;
use crate::email::{self, Email};
use crate::mailbox::Mailbox;

// Error wrapper

#[derive(Debug)]
pub enum Error {
    CreateTlsConnectorError(native_tls::Error),
    CreateImapSession(imap::Error),
    ParseEmailError(mailparse::MailParseError),
    ReadEmailNotFoundError(String),
    ReadEmailEmptyPartError(String, String),
    ExtractAttachmentsEmptyError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(imap): ")?;
        match self {
            Error::CreateTlsConnectorError(err) => err.fmt(f),
            Error::CreateImapSession(err) => err.fmt(f),
            Error::ParseEmailError(err) => err.fmt(f),
            Error::ReadEmailNotFoundError(uid) => {
                write!(f, "no email found for uid {}", uid)
            }
            Error::ReadEmailEmptyPartError(uid, mime) => {
                write!(f, "no {} content found for uid {}", mime, uid)
            }
            Error::ExtractAttachmentsEmptyError(uid) => {
                write!(f, "no attachment found for uid {}", uid)
            }
        }
    }
}

impl From<native_tls::Error> for Error {
    fn from(err: native_tls::Error) -> Error {
        Error::CreateTlsConnectorError(err)
    }
}

impl From<imap::Error> for Error {
    fn from(err: imap::Error) -> Error {
        Error::CreateImapSession(err)
    }
}

impl From<mailparse::MailParseError> for Error {
    fn from(err: mailparse::MailParseError) -> Error {
        Error::ParseEmailError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Imap connector

#[derive(Debug)]
pub struct ImapConnector<'a> {
    pub config: &'a config::ServerInfo,
    pub sess: imap::Session<TlsStream<TcpStream>>,
}

impl<'a> ImapConnector<'a> {
    pub fn new(config: &'a config::ServerInfo) -> Result<Self> {
        let tls = TlsConnector::new()?;
        let client = imap::connect(config.get_addr(), &config.host, &tls)?;
        let sess = client
            .login(&config.login, &config.password)
            .map_err(|res| res.0)?;

        Ok(Self { config, sess })
    }

    pub fn list_mboxes(&mut self) -> Result<Vec<Mailbox<'_>>> {
        let mboxes = self
            .sess
            .list(Some(""), Some("*"))?
            .iter()
            .map(Mailbox::from_name)
            .collect::<Vec<_>>();

        Ok(mboxes)
    }

    pub fn read_emails(&mut self, mbox: &str, query: &str) -> Result<Vec<Email<'_>>> {
        self.sess.select(mbox)?;

        let uids = self
            .sess
            .uid_search(query)?
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();

        let emails = self
            .sess
            .uid_fetch(
                uids[..20.min(uids.len())].join(","),
                "(UID ENVELOPE INTERNALDATE)",
            )?
            .iter()
            .map(Email::from_fetch)
            .collect::<Vec<_>>();

        Ok(emails)
    }

    pub fn read_email_body(&mut self, mbox: &str, uid: &str, mime: &str) -> Result<String> {
        self.sess.select(mbox)?;

        match self.sess.uid_fetch(uid, "BODY[]")?.first() {
            None => Err(Error::ReadEmailNotFoundError(uid.to_string())),
            Some(fetch) => {
                let bytes = fetch.body().unwrap_or(&[]);
                let email = mailparse::parse_mail(bytes)?;
                let bodies = email::extract_text_bodies(&mime, &email);

                if bodies.is_empty() {
                    Err(Error::ReadEmailEmptyPartError(
                        uid.to_string(),
                        mime.to_string(),
                    ))
                } else {
                    Ok(bodies)
                }
            }
        }
    }

    pub fn read_msg(&mut self, mbox: &str, uid: &str) -> Result<Vec<u8>> {
        self.sess.select(mbox)?;

        match self.sess.uid_fetch(uid, "BODY[]")?.first() {
            None => Err(Error::ReadEmailNotFoundError(uid.to_string())),
            Some(fetch) => Ok(fetch.body().unwrap_or(&[]).to_vec()),
        }
    }

    pub fn append_msg(&mut self, mbox: &str, msg: &[u8]) -> Result<()> {
        use imap::types::Flag::*;
        self.sess.append_with_flags(mbox, msg, &[Seen])?;
        Ok(())
    }
}
