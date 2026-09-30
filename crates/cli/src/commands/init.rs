use std::fs;
use std::path::Path;

use crate::error::CliResult;
use crate::prompts;
use crate::templates;
use crate::theme::colors::{AccentColor, BaseColor, generate_theme_vars};

const TAILWIND_PATH: &str = "assets/styles/tailwind.css";
const COLORS_PATH: &str = "assets/styles/colors.css";

#[derive(Debug, Clone, Copy)]
pub struct InitOptions {
    pub yes: bool,
    pub force: bool,
}

pub fn run(opts: InitOptions) -> CliResult<()> {
    run_in(Path::new("."), opts)
}

pub fn run_in(base_dir: &Path, opts: InitOptions) -> CliResult<()> {
    // Спрашиваем цвета только в чисто интерактивном режиме.
    // --yes или --force → дефолты, без промптов.
    let (base, accent) = if opts.yes || opts.force {
        (BaseColor::default(), AccentColor::default())
    } else {
        (
            prompts::prompt_base_color()?,
            prompts::prompt_accent_color()?,
        )
    };

    let tailwind = base_dir.join(TAILWIND_PATH);
    write_with_confirmation(&tailwind, templates::TAILWIND_CSS, opts)?;

    let colors = base_dir.join(COLORS_PATH);
    let colors_css = generate_theme_vars(base, accent);
    write_with_confirmation(&colors, &colors_css, opts)?;

    Ok(())
}

/// Пишет файл, спрашивая подтверждение, если он уже существует.
/// `force = true` — перезаписать без вопросов.
/// `yes = true` — тоже перезаписать без вопросов (дефолты).
fn write_with_confirmation(path: &Path, content: &str, opts: InitOptions) -> CliResult<()> {
    let exists = path.exists();

    if exists && !opts.force && !opts.yes {
        if !prompts::confirm_overwrite(&path.display().to_string())? {
            println!("⏭️  Skipping {}", path.display());
            return Ok(());
        }
    }

    // Не перезаписываем, если содержимое идентично
    if exists {
        if let Ok(existing) = fs::read_to_string(path) {
            if existing == content {
                println!("⏭️  {} is already up to date.", path.display());
                return Ok(());
            }
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    println!("✅ {} written.", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn opts_yes() -> InitOptions {
        InitOptions {
            yes: true,
            force: false,
        }
    }

    fn opts_force() -> InitOptions {
        InitOptions {
            yes: false,
            force: true,
        }
    }

    #[test]
    fn init_yes_creates_both_files() {
        let tmp = TempDir::new().unwrap();
        run_in(tmp.path(), opts_yes()).unwrap();

        assert!(tmp.path().join("assets/styles/tailwind.css").exists());
        assert!(tmp.path().join("assets/styles/colors.css").exists());
    }

    #[test]
    fn init_creates_nested_directories() {
        let tmp = TempDir::new().unwrap();
        run_in(tmp.path(), opts_yes()).unwrap();

        assert!(tmp.path().join("assets").is_dir());
        assert!(tmp.path().join("assets/styles").is_dir());
    }

    #[test]
    fn init_tailwind_contains_imports() {
        let tmp = TempDir::new().unwrap();
        run_in(tmp.path(), opts_yes()).unwrap();

        let css = fs::read_to_string(tmp.path().join("assets/styles/tailwind.css")).unwrap();
        assert!(css.contains(r#"@import "tailwindcss";"#));
        assert!(css.contains(r#"@import "tw-animate-css";"#));
        assert!(css.contains(r#"@import "./colors.css";"#));
        assert!(css.contains("@theme inline"));
        assert!(css.contains("@layer base"));
    }

    #[test]
    fn init_colors_contains_root_and_dark() {
        let tmp = TempDir::new().unwrap();
        run_in(tmp.path(), opts_yes()).unwrap();

        let css = fs::read_to_string(tmp.path().join("assets/styles/colors.css")).unwrap();
        assert!(css.contains(":root {"));
        assert!(css.contains(".dark {"));
        assert!(css.contains("--background:"));
        assert!(css.contains("--destructive:"));
    }

    #[test]
    fn init_yes_uses_default_colors() {
        let tmp = TempDir::new().unwrap();
        run_in(tmp.path(), opts_yes()).unwrap();

        let css = fs::read_to_string(tmp.path().join("assets/styles/colors.css")).unwrap();
        assert!(css.contains("--background: oklch(1 0 0)"));
        assert!(css.contains("--primary: oklch(0.205 0 0)"));
    }

    #[test]
    fn init_force_overwrites_existing() {
        let tmp = TempDir::new().unwrap();

        run_in(tmp.path(), opts_yes()).unwrap();

        let colors_path = tmp.path().join("assets/styles/colors.css");
        fs::write(&colors_path, "GARBAGE").unwrap();

        run_in(tmp.path(), opts_force()).unwrap();

        let css = fs::read_to_string(&colors_path).unwrap();
        assert!(css.contains(":root {"));
        assert!(!css.contains("GARBAGE"));
    }

    #[test]
    fn init_skips_identical_content() {
        let tmp = TempDir::new().unwrap();

        run_in(tmp.path(), opts_yes()).unwrap();
        let first = fs::read_to_string(tmp.path().join("assets/styles/colors.css")).unwrap();

        run_in(tmp.path(), opts_force()).unwrap();
        let second = fs::read_to_string(tmp.path().join("assets/styles/colors.css")).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn init_tailwind_is_static_across_themes() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();

        run_in(tmp_a.path(), opts_yes()).unwrap();
        run_in(tmp_b.path(), opts_yes()).unwrap();

        let tw_a = fs::read_to_string(tmp_a.path().join("assets/styles/tailwind.css")).unwrap();
        let tw_b = fs::read_to_string(tmp_b.path().join("assets/styles/tailwind.css")).unwrap();
        assert_eq!(tw_a, tw_b);
    }
}
