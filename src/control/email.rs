use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::common::{EMAIL_HOST, EMAIL_PASSWORD, EMAIL_USERNAME};

pub struct SendEmail {
    sender: String,
    mailer: SmtpTransport,
}

impl SendEmail {
    /// 创建一个新的邮件发送器
    pub fn new() -> Self {
        let creds = Credentials::new(EMAIL_USERNAME.to_owned(), EMAIL_PASSWORD.to_owned());

        // Open a remote connection to 163
        let mailer = SmtpTransport::relay(EMAIL_HOST)
            .unwrap()
            .credentials(creds)
            .build();

        Self {
            sender: EMAIL_USERNAME.to_string(),
            mailer,
        }
    }

    /// 发送邮件 subject 标题，body 正文，email_address 收件人地址
    pub fn send(&self, subject: &str, body: &str, email_address: &str) -> anyhow::Result<()> {
        let email = Message::builder()
            .from(self.sender.parse()?)
            .to(email_address.parse()?);

        let message = email
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())?;

        // Send the email
        let _ = &self.mailer.send(&message)?;
        Ok(())
    }
}
