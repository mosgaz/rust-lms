use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Select};

use crate::error::CliResult;
use crate::theme::colors::{AccentColor, BaseColor};

pub fn prompt_base_color() -> CliResult<BaseColor> {
    let labels = BaseColor::all_labels();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Base color")
        .default(0)
        .items(&labels)
        .interact()?;
    Ok(BaseColor::from_index(selection))
}

pub fn prompt_accent_color() -> CliResult<AccentColor> {
    let labels = AccentColor::all_labels();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Accent color")
        .default(0)
        .items(&labels)
        .interact()?;
    Ok(AccentColor::from_index(selection))
}

pub fn confirm_overwrite(path: &str) -> CliResult<bool> {
    Ok(Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("{path} already exists. Overwrite?"))
        .default(false)
        .interact()?)
}