// crates/client/src/admin/routes.rs
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Clone, Copy, Display, AsRefStr, IntoStaticStr, EnumString, EnumIter, Debug, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum AdminRoutes {
    Analytics,
    Dashboard,
    Features,
    Batches,
    Certificates,
    Courses,
    Tests,
    Users,
	Licenses,
	Profile,
    Settings,
    Tenants,
}

impl AdminRoutes {
    pub fn to_route(self) -> String {
        match self {
            Self::Dashboard => "/admin".to_string(),
            other => format!("/admin/{}", other.as_ref()),
        }
    }

    pub fn to_title(self) -> &'static str {
        match self {
            Self::Analytics => "Аналитика",
            Self::Dashboard => "Дашборд",
            Self::Features => "Функционал",
            Self::Certificates => "Сертификаты",
            Self::Batches => "Потоки",
            Self::Courses => "Курсы",
            Self::Tests => "Тесты",
            Self::Users => "Пользователи",
            Self::Tenants => "Организации",
            Self::Profile => "Профиль",
            Self::Licenses => "Лицензии",
            Self::Settings => "Настройки",
        }
    }
}