// crates/client/src/admin/routes.rs
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Clone, Copy, Display, AsRefStr, IntoStaticStr, EnumString, EnumIter, Debug, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum AdminRoutes {
    Dashboard,
    Courses,
    Tests,
    Users,
	Profile,
    Settings,
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
            Self::Dashboard => "Дашборд",
            Self::Courses => "Курсы",
            Self::Tests => "Тесты",
            Self::Users => "Пользователи",
            Self::Profile => "Профиль",
            Self::Settings => "Настройки",
        }
    }
}