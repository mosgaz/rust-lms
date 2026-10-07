// crates/api/src/services/email.rs
//! Сервис для отправки электронных писем через SMTP.
//!
//! Использует крейт `lettre` для формирования и отправки MIME-писем
//! с поддержкой вложений (PDF-сертификаты).

use anyhow::{Context, Result};
use lettre::{
    message::{header::ContentType, Attachment, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

/// Конфигурация SMTP-подключения.
#[derive(Debug, Clone)]
pub struct SmtpSettings {
    /// Адрес SMTP-сервера.
    pub host: String,
    /// Порт SMTP-сервера.
    pub port: u16,
    /// Имя пользователя для аутентификации.
    pub username: String,
    /// Пароль для аутентификации.
    pub password: String,
    /// Email-адрес отправителя.
    pub from_address: String,
    /// Отображаемое имя отправителя.
    pub from_name: String,
}

/// Сервис для отправки электронных писем.
#[derive(Clone)]
pub struct EmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_address: String,
    from_name: String,
}

impl EmailService {
    /// Создаёт новый экземпляр email-сервиса с указанными настройками SMTP.
    pub fn new(settings: SmtpSettings) -> Result<Self> {
        let creds = Credentials::new(settings.username.clone(), settings.password.clone());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.host)
            .context("Не удалось создать SMTP-транспорт")?
            .port(settings.port)
            .credentials(creds)
            .build();

        Ok(Self {
            mailer,
            from_address: settings.from_address,
            from_name: settings.from_name,
        })
    }

    /// Создаёт заглушку email-сервиса для сред без SMTP.
    pub fn new_stub() -> Self {
        Self {
            mailer: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("localhost").build(),
            from_address: "noreply@localhost".to_string(),
            from_name: "LMS Stub".to_string(),
        }
    }

    /// Отправляет письмо с PDF-сертификатом во вложении.
    pub async fn send_certificate_email(
        &self,
        to_email: &str,
        to_name: &str,
        course_title: &str,
        pdf_bytes: Vec<u8>,
        verification_hash: &str,
    ) -> Result<()> {
        let from = format!("{} <{}>", self.from_name, self.from_address)
            .parse()
            .context("Невалидный адрес отправителя")?;
        let to = format!("{} <{}>", to_name, to_email)
            .parse()
            .context("Невалидный адрес получателя")?;

        let subject = format!("Ваш сертификат: {}", course_title);

        let body_text = format!(
            "Здравствуйте, {}!\n\n\
             Поздравляем с успешным завершением курса «{}»!\n\n\
             Ваш сертификат прикреплён к этому письму в формате PDF.\n\n\
             Код верификации: {}\n\
             Проверить подлинность: https://lms.example.com/verify/{}\n\n\
             С уважением,\n\
             Команда LMS",
            to_name, course_title, verification_hash, verification_hash
        );

        let attachment = Attachment::new("certificate.pdf".to_string())
            .body(pdf_bytes, ContentType::parse("application/pdf").unwrap());

        let email = Message::builder()
            .from(from)
            .to(to)
            .subject(&subject)
            .multipart(
                MultiPart::mixed()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(body_text),
                    )
                    .singlepart(attachment),
            )
            .context("Не удалось сформировать MIME-письмо")?;

        self.mailer
            .send(email)
            .await
            .context("Не удалось отправить письмо через SMTP")?;

        tracing::info!(
            to_email = %to_email,
            course = %course_title,
            "Certificate email sent successfully"
        );

        Ok(())
    }
}