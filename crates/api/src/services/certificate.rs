// crates/api/src/services/certificate.rs
//! Сервис для управления жизненным циклом сертификатов.
//!
//! Инкапсулирует бизнес-логику выдачи, проверки, получения сертификатов,
//! генерации PDF-документов и отправки уведомлений по электронной почте.

use crate::database::{
    Certificate, CertificateRepository, CourseRepository, IdentityRepository, RlsContext,
    UserRepository,
};
use crate::services::email::EmailService;
use anyhow::{Context, Result};
use printpdf::*;
use qrcode::QrCode;
use rust_lms_shared::{CourseId, UserId};
use sqlx::PgPool;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use uuid::Uuid;

/// Сервис для управления жизненным циклом сертификатов.
#[derive(Clone)]
pub struct CertificateService {
    repo: CertificateRepository,
    user_repo: UserRepository,
    course_repo: CourseRepository,
    identity_repo: IdentityRepository,
    email_service: Option<EmailService>,
}

impl CertificateService {
    /// Создаёт новый экземпляр сервиса сертификатов с поддержкой email-уведомлений.
    pub fn new(pool: PgPool, email_service: Option<EmailService>) -> Self {
        Self {
            repo: CertificateRepository::new(pool.clone()),
            user_repo: UserRepository::new(pool.clone()),
            course_repo: CourseRepository::new(pool.clone()),
            identity_repo: IdentityRepository::new(pool),
            email_service,
        }
    }

    /// Выдаёт сертификат за успешное завершение курса.
    ///
    /// Создаёт запись в БД, генерирует PDF и отправляет уведомление на email студента.
    pub async fn issue_course_certificate(
        &self,
        ctx: &RlsContext,
        user_id: Uuid,
        course_id: Uuid,
    ) -> Result<Certificate> {
        let tenant_id = ctx.tenant_id();
        let uid = UserId(user_id);
        let cid = CourseId(course_id);

        // 1. Создаём запись сертификата в БД
        let cert = self.repo.create(ctx, user_id, "course", course_id).await?;

        // 2. Получаем User для доступа к identity_id
        let user = self
            .user_repo
            .find_by_id(tenant_id, uid)
            .await
            .context("Не удалось найти пользователя для сертификата")?;

        // 3. Получаем Identity для доступа к email
        let identity = self
            .identity_repo
            .find_by_id(user.identity_id)
            .await
            .context("Не удалось найти identity для сертификата")?;

        // 4. Получаем Course для доступа к title
        let course = self
            .course_repo
            .find_by_id(tenant_id, cid)
            .await
            .context("Не удалось найти курс для сертификата")?;

        let user_name = identity.email.as_str();

        // 5. Генерируем PDF
        let pdf_bytes = self.generate_certificate_pdf(&cert, user_name, &course.title)?;

        // 6. Отправляем email (если SMTP настроен)
        if let Some(ref email_svc) = self.email_service {
            if let Err(e) = email_svc
                .send_certificate_email(
                    &identity.email,
                    user_name,
                    &course.title,
                    pdf_bytes,
                    &cert.verification_hash,
                )
                .await
            {
                tracing::error!(
                    error = %e,
                    user_id = %user_id,
                    "Failed to send certificate email, certificate was still issued"
                );
            }
        }

        Ok(cert)
    }

    /// Генерирует PDF-файл сертификата.
    pub fn generate_certificate_pdf(
        &self,
        cert: &Certificate,
        user_name: &str,
        course_title: &str,
    ) -> Result<Vec<u8>> {
        let (doc, page1, layer1) =
            PdfDocument::new("Certificate", Mm(210.0), Mm(297.0), "Layer 1");
        let current_layer = doc.get_page(page1).get_layer(layer1);

        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let font_path = Path::new(&manifest_dir).join("assets/fonts/DejaVuSans.ttf");

        let font_file = File::open(&font_path).context(format!(
            "Не удалось открыть файл шрифта по пути {:?}.",
            font_path
        ))?;

        let font = doc
            .add_external_font(font_file)
            .context("Не удалось загрузить шрифт в PDF-документ")?;

        // 1. Рамка сертификата
        let border_rect = Rect::new(Mm(10.0), Mm(10.0), Mm(190.0), Mm(277.0));
        current_layer.set_outline_color(Color::Rgb(Rgb::new(0.2, 0.2, 0.2, None)));
        current_layer.set_outline_thickness(2.0);
        current_layer.add_rect(border_rect);

        // 2. Заголовок
        current_layer.use_text("СЕРТИФИКАТ", 36.0, Mm(55.0), Mm(250.0), &font);

        // 3. Основной текст
        current_layer.begin_text_section();
        current_layer.set_font(&font, 16.0);
        current_layer.set_text_cursor(Mm(20.0), Mm(220.0));
        current_layer.set_line_height(20.0);

        current_layer.write_text("Настоящим подтверждается, что", &font);
        current_layer.add_line_break();

        current_layer.set_font(&font, 24.0);
        current_layer.set_text_cursor(Mm(20.0), Mm(190.0));
        current_layer.write_text(user_name, &font);
        current_layer.add_line_break();

        current_layer.set_font(&font, 16.0);
        current_layer.set_text_cursor(Mm(20.0), Mm(160.0));
        current_layer.write_text("успешно завершил(а) курс:", &font);
        current_layer.add_line_break();

        current_layer.set_font(&font, 20.0);
        current_layer.set_text_cursor(Mm(20.0), Mm(130.0));
        current_layer.write_text(course_title, &font);
        current_layer.end_text_section();

        // 4. Дата выдачи
        let date_str = cert.issued_at.format("%d.%m.%Y").to_string();
        current_layer.begin_text_section();
        current_layer.set_font(&font, 12.0);
        current_layer.set_text_cursor(Mm(20.0), Mm(50.0));
        current_layer.write_text(&format!("Дата выдачи: {}", date_str), &font);
        current_layer.end_text_section();

        // 5. QR-код для публичной верификации
        let verification_url = format!(
            "https://lms.example.com/verify/{}",
            cert.verification_hash
        );
        let code = QrCode::new(verification_url).context("Не удалось сгенерировать QR-код")?;

        let modules = code.width();
        let module_size = 0.4;
        let start_x = 140.0;
        let start_y = 40.0;

        current_layer.set_fill_color(Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));

        for y in 0..modules {
            for x in 0..modules {
                if code[(x, y)] == qrcode::Color::Dark {
                    let rect = Rect::new(
                        Mm(start_x + (x as f32 * module_size)),
                        Mm(start_y + (y as f32 * module_size)),
                        Mm(module_size),
                        Mm(module_size),
                    );
                    current_layer.add_rect(rect);
                }
            }
        }

        let mut buffer = Vec::new();
        {
            let mut writer = BufWriter::new(&mut buffer);
            doc.save(&mut writer)
                .context("Не удалось сохранить PDF в буфер")?;
            writer.flush().context("Не удалось сбросить буфер PDF")?;
        }

        Ok(buffer)
    }

    /// Проверяет подлинность сертификата по публичному хешу верификации.
    pub async fn verify_certificate(&self, hash: &str) -> Result<Option<Certificate>> {
        self.repo.find_by_verification_hash(hash).await
    }

    /// Возвращает список всех сертификатов конкретного пользователя.
    pub async fn get_user_certificates(
        &self,
        ctx: &RlsContext,
        user_id: Uuid,
    ) -> Result<Vec<Certificate>> {
        self.repo.list_by_user(ctx, user_id).await
    }

	/// Получает сертификат с данными пользователя и курса для генерации PDF.
    ///
    /// # Arguments
    /// * `ctx` - Контекст тенанта.
    /// * `certificate_id` - Идентификатор сертификата.
    ///
    /// # Returns
    /// Кортеж (сертификат, имя пользователя, название курса) или ошибку.
    pub async fn get_certificate_with_details(
        &self,
        ctx: &RlsContext,
        certificate_id: Uuid,
    ) -> Result<(Certificate, String, String)> {
        let cert = self
            .repo
            .find_by_id(ctx, certificate_id)
            .await?
            .context("Сертификат не найден")?;

        let tenant_id = ctx.tenant_id();
        let uid = UserId(cert.user_id);
        let cid = CourseId(cert.target_id);

        let user = self
            .user_repo
            .find_by_id(tenant_id, uid)
            .await
            .context("Не удалось найти пользователя")?;

        let identity = self
            .identity_repo
            .find_by_id(user.identity_id)
            .await
            .context("Не удалось найти identity")?;

        let course = self
            .course_repo
            .find_by_id(tenant_id, cid)
            .await
            .context("Не удалось найти курс")?;

        Ok((cert, identity.email.clone(), course.title))
    }
	
}