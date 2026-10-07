// crates/api/src/services/certificate.rs
//! Сервис для управления жизненным циклом сертификатов.
//!
//! Инкапсулирует бизнес-логику выдачи, проверки, получения сертификатов
//! и генерации PDF-документов.

use crate::database::{Certificate, CertificateRepository, CourseRepository, RlsContext, UserRepository};
use anyhow::{Context, Result};
use printpdf::*;
use qrcode::QrCode;
use sqlx::PgPool;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use uuid::Uuid;

/// Сервис для управления жизненным циклом сертификатов.
#[derive(Clone)]
pub struct CertificateService {
    repo: CertificateRepository,
    /// Репозиторий пользователей. Будет использован в Подэтапе 11.3 для получения email студента.
    #[allow(dead_code)]
    user_repo: UserRepository,
    /// Репозиторий курсов. Будет использован в Подэтапе 11.3 для получения названия курса.
    #[allow(dead_code)]
    course_repo: CourseRepository,
}

impl CertificateService {
    /// Создаёт новый экземпляр сервиса сертификатов.
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: CertificateRepository::new(pool.clone()),
            user_repo: UserRepository::new(pool.clone()),
            course_repo: CourseRepository::new(pool),
        }
    }

    /// Выдаёт сертификат за успешное завершение курса.
    pub async fn issue_course_certificate(
        &self,
        ctx: &RlsContext,
        user_id: Uuid,
        course_id: Uuid,
    ) -> Result<Certificate> {
        self.repo.create(ctx, user_id, "course", course_id).await
    }

    /// Генерирует PDF-файл сертификата.
    ///
    /// # Arguments
    /// * `cert` - Модель выданного сертификата.
    /// * `user_name` - Полное имя пользователя для отображения в сертификате.
    /// * `course_title` - Название курса для отображения в сертификате.
    ///
    /// # Returns
    /// Вектор байтов, содержащий сгенерированный PDF-документ.
    pub fn generate_certificate_pdf(
        &self,
        cert: &Certificate,
        user_name: &str,
        course_title: &str,
    ) -> Result<Vec<u8>> {
        // Инициализация PDF-документа (A4, портретная ориентация)
        // ИСПРАВЛЕНО: убран `mut` у `doc`, так как он не изменяется напрямую
        let (doc, page1, layer1) =
            PdfDocument::new("Certificate", Mm(210.0), Mm(297.0), "Layer 1");
        let current_layer = doc.get_page(page1).get_layer(layer1);

        // Загрузка шрифта из локальной директории assets
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let font_path = Path::new(&manifest_dir).join("assets/fonts/DejaVuSans.ttf");
        
        let font_file = File::open(&font_path).context(format!(
            "Не удалось открыть файл шрифта по пути {:?}. Пожалуйста, поместите файл шрифта (например, DejaVuSans.ttf) в эту директорию.",
            font_path
        ))?;
        
        let font = doc.add_external_font(font_file).context("Не удалось загрузить шрифт в PDF-документ")?;

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
        let verification_url = format!("https://lms.example.com/verify/{}", cert.verification_hash);
        let code = QrCode::new(verification_url).context("Не удалось сгенерировать QR-код")?;
        
        let modules = code.width();
        let module_size = 0.4; // мм на один модуль QR-кода
        let start_x = 140.0;   // мм от левого края
        let start_y = 40.0;    // мм от нижнего края

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

        // Сохранение в вектор байтов через BufWriter
        let mut buffer = Vec::new();
        
        // ИСПРАВЛЕНО: Ограничиваем область видимости writer, чтобы освободить заимствование buffer
        {
            let mut writer = BufWriter::new(&mut buffer);
            doc.save(&mut writer).context("Не удалось сохранить PDF в буфер")?;
            writer.flush().context("Не удалось сбросить буфер PDF")?;
        } // writer уничтожается здесь, заимствование buffer завершается
        
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
}