// crates/api/tests/certificate_integration.rs
//! Интеграционные тесты для Certification Engine (Этап 11).
//!
//! Проверяют полный цикл работы с сертификатами:
//! - Выдача при завершении курса
//! - Генерация PDF
//! - Публичная верификация
//! - Скачивание PDF
//! - Отправка email-уведомлений

use rust_lms_api::{
    database::{CertificateRepository, DatabasePool, RlsContext},
    services::{CertificateService, EmailService, SmtpSettings},
};
use rust_lms_shared::{TenantId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

/// Создаёт тестовый пул соединений с миграциями.
async fn setup_test_pool() -> PgPool {
    let pool = PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .expect("failed to connect to test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    pool
}

/// Создаёт тестовые данные: тенант, пользователь, курс.
async fn create_test_data(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let course_id = Uuid::new_v4();

    // Создаём тенант
    sqlx::query("INSERT INTO tenants (id, name, slug, is_active) VALUES ($1, 'Test Tenant', 'test', true)")
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("failed to create test tenant");

    // Создаём identity
    let identity_id = Uuid::new_v4();
    sqlx::query("INSERT INTO identities (id, email, password_hash) VALUES ($1, 'test@example.com', 'hash')")
        .bind(identity_id)
        .execute(pool)
        .await
        .expect("failed to create test identity");

    // Создаём пользователя
    sqlx::query("INSERT INTO users (id, tenant_id, identity_id, is_active) VALUES ($1, $2, $3, true)")
        .bind(user_id)
        .bind(tenant_id)
        .bind(identity_id)
        .execute(pool)
        .await
        .expect("failed to create test user");

    // Создаём курс
    sqlx::query("INSERT INTO courses (id, tenant_id, title, version) VALUES ($1, $2, 'Test Course', 1)")
        .bind(course_id)
        .bind(tenant_id)
        .execute(pool)
        .await
        .expect("failed to create test course");

    (tenant_id, user_id, course_id)
}

#[tokio::test]
async fn test_certificate_creation() {
    let pool = setup_test_pool().await;
    let (tenant_id, user_id, course_id) = create_test_data(&pool).await;

    let repo = CertificateRepository::new(pool.clone());
    let ctx = RlsContext::new(TenantId(tenant_id));

    // Создаём сертификат
    let cert = repo
        .create(&ctx, user_id, "course", course_id)
        .await
        .expect("failed to create certificate");

    assert_eq!(cert.user_id, user_id);
    assert_eq!(cert.target_id, course_id);
    assert_eq!(cert.target_type, "course");
    assert!(!cert.verification_hash.is_empty());
}

#[tokio::test]
async fn test_certificate_verification_by_hash() {
    let pool = setup_test_pool().await;
    let (tenant_id, user_id, course_id) = create_test_data(&pool).await;

    let repo = CertificateRepository::new(pool.clone());
    let ctx = RlsContext::new(TenantId(tenant_id));

    let cert = repo
        .create(&ctx, user_id, "course", course_id)
        .await
        .expect("failed to create certificate");

    // Проверяем верификацию по хешу
    let found = repo
        .find_by_verification_hash(&cert.verification_hash)
        .await
        .expect("failed to find certificate")
        .expect("certificate not found");

    assert_eq!(found.id, cert.id);
    assert_eq!(found.verification_hash, cert.verification_hash);
}

#[tokio::test]
async fn test_certificate_list_by_user() {
    let pool = setup_test_pool().await;
    let (tenant_id, user_id, course_id) = create_test_data(&pool).await;

    let repo = CertificateRepository::new(pool.clone());
    let ctx = RlsContext::new(TenantId(tenant_id));

    // Создаём два сертификата
    repo.create(&ctx, user_id, "course", course_id)
        .await
        .expect("failed to create certificate 1");

    let course_id_2 = Uuid::new_v4();
    sqlx::query("INSERT INTO courses (id, tenant_id, title, version) VALUES ($1, $2, 'Test Course 2', 1)")
        .bind(course_id_2)
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("failed to create test course 2");

    repo.create(&ctx, user_id, "course", course_id_2)
        .await
        .expect("failed to create certificate 2");

    // Получаем список
    let certs = repo
        .list_by_user(&ctx, user_id)
        .await
        .expect("failed to list certificates");

    assert_eq!(certs.len(), 2);
}

#[tokio::test]
async fn test_pdf_generation() {
    let pool = setup_test_pool().await;
    let (tenant_id, user_id, course_id) = create_test_data(&pool).await;

    let service = CertificateService::new(pool.clone(), None);
    let ctx = RlsContext::new(TenantId(tenant_id));

    let cert = service
        .issue_course_certificate(&ctx, user_id, course_id)
        .await
        .expect("failed to issue certificate");

    // Генерируем PDF
    let pdf_bytes = service
        .generate_certificate_pdf(&cert, "Test User", "Test Course")
        .expect("failed to generate PDF");

    // Проверяем, что PDF не пустой и начинается с заголовка PDF
    assert!(!pdf_bytes.is_empty());
    assert!(pdf_bytes.starts_with(b"%PDF"));
}

#[tokio::test]
async fn test_email_service_stub() {
    // Создаём мок email-сервиса
    let email_service = EmailService::new_stub();

    // Отправляем тестовое письмо (должно завершиться успешно, но ничего не отправить)
    let result = email_service
        .send_certificate_email(
            "test@example.com",
            "Test User",
            "Test Course",
            vec![1, 2, 3],
            "test-hash",
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_certificate_service_with_email() {
    let pool = setup_test_pool().await;
    let (tenant_id, user_id, course_id) = create_test_data(&pool).await;

    // Создаём сервис с моком email
    let email_service = EmailService::new_stub();
    let service = CertificateService::new(pool.clone(), Some(email_service));
    let ctx = RlsContext::new(TenantId(tenant_id));

    // Выдаём сертификат (должен попытаться отправить email, но без ошибок)
    let cert = service
        .issue_course_certificate(&ctx, user_id, course_id)
        .await
        .expect("failed to issue certificate");

    assert_eq!(cert.user_id, user_id);
    assert_eq!(cert.target_id, course_id);
}

#[tokio::test]
async fn test_rls_isolation() {
    let pool = setup_test_pool().await;

    // Создаём два тенанта
    let tenant_id_1 = Uuid::new_v4();
    let tenant_id_2 = Uuid::new_v4();

    sqlx::query("INSERT INTO tenants (id, name, slug, is_active) VALUES ($1, 'Tenant 1', 't1', true)")
        .bind(tenant_id_1)
        .execute(&pool)
        .await
        .expect("failed to create tenant 1");

    sqlx::query("INSERT INTO tenants (id, name, slug, is_active) VALUES ($1, 'Tenant 2', 't2', true)")
        .bind(tenant_id_2)
        .execute(&pool)
        .await
        .expect("failed to create tenant 2");

    // Создаём пользователей
    let user_id_1 = Uuid::new_v4();
    let user_id_2 = Uuid::new_v4();

    let identity_id_1 = Uuid::new_v4();
    let identity_id_2 = Uuid::new_v4();

    sqlx::query("INSERT INTO identities (id, email, password_hash) VALUES ($1, 'user1@example.com', 'hash')")
        .bind(identity_id_1)
        .execute(&pool)
        .await
        .expect("failed to create identity 1");

    sqlx::query("INSERT INTO identities (id, email, password_hash) VALUES ($1, 'user2@example.com', 'hash')")
        .bind(identity_id_2)
        .execute(&pool)
        .await
        .expect("failed to create identity 2");

    sqlx::query("INSERT INTO users (id, tenant_id, identity_id, is_active) VALUES ($1, $2, $3, true)")
        .bind(user_id_1)
        .bind(tenant_id_1)
        .bind(identity_id_1)
        .execute(&pool)
        .await
        .expect("failed to create user 1");

    sqlx::query("INSERT INTO users (id, tenant_id, identity_id, is_active) VALUES ($1, $2, $3, true)")
        .bind(user_id_2)
        .bind(tenant_id_2)
        .bind(identity_id_2)
        .execute(&pool)
        .await
        .expect("failed to create user 2");

    // Создаём курсы
    let course_id_1 = Uuid::new_v4();
    let course_id_2 = Uuid::new_v4();

    sqlx::query("INSERT INTO courses (id, tenant_id, title, version) VALUES ($1, $2, 'Course 1', 1)")
        .bind(course_id_1)
        .bind(tenant_id_1)
        .execute(&pool)
        .await
        .expect("failed to create course 1");

    sqlx::query("INSERT INTO courses (id, tenant_id, title, version) VALUES ($1, $2, 'Course 2', 1)")
        .bind(course_id_2)
        .bind(tenant_id_2)
        .execute(&pool)
        .await
        .expect("failed to create course 2");

    let repo = CertificateRepository::new(pool.clone());

    // Создаём сертификаты в разных тенантах
    let ctx_1 = RlsContext::new(TenantId(tenant_id_1));
    let ctx_2 = RlsContext::new(TenantId(tenant_id_2));

    repo.create(&ctx_1, user_id_1, "course", course_id_1)
        .await
        .expect("failed to create certificate in tenant 1");

    repo.create(&ctx_2, user_id_2, "course", course_id_2)
        .await
        .expect("failed to create certificate in tenant 2");

    // Проверяем изоляцию: каждый тенант видит только свои сертификаты
    let certs_1 = repo
        .list_by_user(&ctx_1, user_id_1)
        .await
        .expect("failed to list certificates for tenant 1");

    let certs_2 = repo
        .list_by_user(&ctx_2, user_id_2)
        .await
        .expect("failed to list certificates for tenant 2");

    assert_eq!(certs_1.len(), 1);
    assert_eq!(certs_2.len(), 1);

    // Пытаемся получить сертификат из другого тенанта (должен вернуть None)
    let cert_1 = repo
        .find_by_id(&ctx_1, certs_2[0].id)
        .await
        .expect("failed to query certificate");

    assert!(cert_1.is_none(), "RLS should prevent cross-tenant access");
}