// crates/client/src/student/routes.rs
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

#[derive(Clone, Copy, Display, AsRefStr, IntoStaticStr, EnumString, EnumIter, Debug, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum StudentRoutes {
    Dashboard,
    MyCourses,
    Schedule,
    Grades,
	Profile,
    Settings,
}

impl StudentRoutes {
    pub fn to_route(self) -> String {
        match self {
            Self::Dashboard => "/student".to_string(),
            other => format!("/student/{}", other.as_ref()),
        }
    }

    pub fn to_title(self) -> &'static str {
        match self {
            Self::Dashboard => "Дашборд",
            Self::MyCourses => "Мои курсы",
            Self::Schedule => "Расписание",
            Self::Grades => "Оценки",
            Self::Profile => "Профиль",
            Self::Settings => "Настройки",
        }
    }
}