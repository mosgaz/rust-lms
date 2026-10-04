// crates/ui/src/components/toast.rs
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use leptos::prelude::*;

/* ========================================================== */
/*                       🧬 DATA 🧬                           */
/* ========================================================== */

pub type ToastId = u64;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ToastLevel {
    Info,
    Success,
    Warn,
    Error,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ToastPosition {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

#[derive(Clone, Debug)]
pub struct ToastData {
    pub id: ToastId,
    pub expiry: Option<u32>,
    pub message: String,
    pub progress: bool,
    pub dismissable: bool,
    pub clear_signal: RwSignal<bool>,

    pub level: ToastLevel,
    pub position: ToastPosition,
}

/* ========================================================== */
/*                     🧬 BUILDER 🧬                          */
/* ========================================================== */

pub struct ToastBuilder {
    message: String,
    level: ToastLevel,
    dismissable: bool,
    expiry: Option<u32>,
    progress: bool,
    position: ToastPosition,
}

impl ToastBuilder {
    #[must_use]
    pub fn new<T>(message: T) -> Self
    where
        T: Display,
    {
        ToastBuilder {
            progress: true,
            dismissable: true,
            expiry: Some(2_500),
            level: ToastLevel::Info,
            message: message.to_string(),
            position: ToastPosition::BottomRight,
        }
    }

    #[must_use]
    pub fn with_level(mut self, level: ToastLevel) -> Self {
        self.level = level;
        self
    }

    #[must_use]
    pub fn with_dismissable(mut self, dismissable: bool) -> Self {
        self.dismissable = dismissable;
        self
    }

    #[must_use]
    pub fn with_progress(mut self, progress: bool) -> Self {
        self.progress = progress;
        self
    }

    #[must_use]
    pub fn with_expiry(mut self, expiry: Option<u32>) -> Self {
        self.expiry = expiry;
        self
    }

    #[must_use]
    pub fn with_position(mut self, position: ToastPosition) -> Self {
        self.position = position;
        self
    }

    /// Builds the toast into a `ToastData` with the supplied ID.
    #[must_use]
    pub fn build(self, id: ToastId) -> ToastData {
        ToastData {
            id,
            level: self.level,
            expiry: self.expiry,
            message: self.message,
            position: self.position,
            progress: self.progress,
            dismissable: self.dismissable,
            clear_signal: RwSignal::new(false),
        }
    }
}

/* ========================================================== */
/*                    🧬 CONTEXT 🧬                           */
/* ========================================================== */

#[derive(Clone, Debug)]
pub struct ToasterContext {
    stats: Arc<Mutex<ToasterStats>>,
    pub queue_signal: RwSignal<Vec<ToastData>>,
}

#[derive(Clone, Default, Debug)]
struct ToasterStats {
    visible: u32,
    total: u64,
}

impl ToasterContext {
    pub fn toast(&self, builder: ToastBuilder) {
        let Ok(mut stats) = self.stats.lock() else { return };
        let stats = &mut *stats;
        let toast = builder.build(stats.total + 1);

        let mut queue = self.queue_signal.get_untracked();
        queue.push(toast);
        self.queue_signal.set(queue);
        stats.visible += 1;
        stats.total += 1;
    }

    pub fn info<T>(&self, message: T)
    where
        T: Display,
    {
        self.toast(ToastBuilder::new(message).with_level(ToastLevel::Info));
    }

    pub fn success<T>(&self, message: T)
    where
        T: Display,
    {
        self.toast(ToastBuilder::new(message).with_level(ToastLevel::Success));
    }

    pub fn warn<T>(&self, message: T)
    where
        T: Display,
    {
        self.toast(ToastBuilder::new(message).with_level(ToastLevel::Warn));
    }

    pub fn error<T>(&self, message: T)
    where
        T: Display,
    {
        self.toast(ToastBuilder::new(message).with_level(ToastLevel::Error));
    }

    pub fn clear(&self) {
        for toast in &self.queue_signal.get_untracked() {
            toast.clear_signal.set(true);
        }
    }

    /// Removes the toast corresponding with the supplied `ToastId`.
    pub fn remove(&self, toast_id: ToastId) {
        let index = self
            .queue_signal
            .get_untracked()
            .iter()
            .enumerate()
            .find(|(_, toast)| toast.id == toast_id)
            .map(|(index, _)| index);

        if let Some(index) = index {
            let mut queue = self.queue_signal.get_untracked();
            queue.remove(index);
            self.queue_signal.set(queue);

            if let Ok(mut stats) = self.stats.lock() {
                stats.visible -= 1;
            }
        }
    }
}

impl Default for ToasterContext {
    fn default() -> Self {
        ToasterContext {
            stats: Arc::new(Mutex::new(ToasterStats::default())),
            queue_signal: RwSignal::new(Vec::new()),
        }
    }
}

/* ========================================================== */
/*                   🧬 TEMPLATE STYLES 🧬                    */
/* ========================================================== */

pub const TEMPLATE_STYLES: &str = r#"
:root {
    --leptoaster-width: 320px;
    --leptoaster-max-width: 80vw;
    --leptoaster-z-index: 9999;

    --leptoaster-font-family: Arial;
    --leptoaster-font-size: 14px;
    --leptoaster-line-height: 20px;
    --leptoaster-font-weight: 600;

    --leptoaster-progress-height: 2px;

    --leptoaster-info-background-color: #ffffff;
    --leptoaster-info-border-color: #222222;
    --leptoaster-info-text-color: #222222;

    --leptoaster-success-background-color: #4caf50;
    --leptoaster-success-border-color: #2e7d32;
    --leptoaster-success-text-color: #ffffff;

    --leptoaster-warn-background-color: #ff9800;
    --leptoaster-warn-border-color: #ff8f00;
    --leptoaster-warn-text-color: #ffffff;

    --leptoaster-error-background-color: #f44336;
    --leptoaster-error-border-color: #c62828;
    --leptoaster-error-text-color: #ffffff;
}

.leptoaster-stack-container-bottom:hover > div,
.leptoaster-stack-container-top:hover > div {
    opacity: 1 !important;
    transform: translateY(0) scaleX(1) !important;
    transition-delay: 0s !important;
}

.leptoaster-stack-container-bottom > div:nth-last-child(1),
.leptoaster-stack-container-top > div:nth-child(1) {
    z-index: 9999;
}

.leptoaster-stack-container-bottom > div:nth-last-child(2),
.leptoaster-stack-container-top > div:nth-child(2) {
    z-index: 9998;
}

.leptoaster-stack-container-bottom > div:nth-last-child(2) {
    transform: translateY(62px) scaleX(0.98);
}

.leptoaster-stack-container-top > div:nth-child(2) {
    transform: translateY(-62px) scaleX(0.98);
}

.leptoaster-stack-container-bottom > div:nth-last-child(3),
.leptoaster-stack-container-top > div:nth-child(3) {
    z-index: 9997;
}

.leptoaster-stack-container-bottom > div:nth-last-child(3) {
    transform: translateY(124px) scaleX(0.96);
}

.leptoaster-stack-container-top > div:nth-child(3) {
    transform: translateY(-124px) scaleX(0.96);
}

.leptoaster-stack-container-bottom > div:nth-last-child(4),
.leptoaster-stack-container-top > div:nth-child(4) {
    z-index: 9996;
}

.leptoaster-stack-container-bottom > div:nth-last-child(4) {
    transform: translateY(186px) scaleX(0.94);
}

.leptoaster-stack-container-top > div:nth-child(4) {
    transform: translateY(-186px) scaleX(0.94);
}

.leptoaster-stack-container-bottom > div:nth-last-child(5),
.leptoaster-stack-container-top > div:nth-child(5) {
    z-index: 9995;
}

.leptoaster-stack-container-bottom > div:nth-last-child(5) {
    transform: translateY(248px) scaleX(0.92);
}

.leptoaster-stack-container-top > div:nth-child(5) {
    transform: translateY(-248px) scaleX(0.92);
}

.leptoaster-stack-container-bottom > div:nth-last-child(n+6),
.leptoaster-stack-container-top > div:nth-child(n+6) {
    opacity: 0;
}

@keyframes leptoaster-slide-in-left {
    from { left: calc((var(--leptoaster-width) + 12px * 2) * -1) }
    to { left: 0 }
}

@keyframes leptoaster-slide-out-left {
    from { left: 0 }
    to { left: calc((var(--leptoaster-width) + 12px * 2) * -1) }
}

@keyframes leptoaster-slide-in-right {
    from { right: calc((var(--leptoaster-width) + 12px * 2) * -1) }
    to { right: 0 }
}

@keyframes leptoaster-slide-out-right {
    from { right: 0 }
    to { right: calc((var(--leptoaster-width) + 12px * 2) * -1) }
}

@keyframes leptoaster-progress {
    from { width: 100%; }
    to { width: 0; }
}
"#;

/* ========================================================== */
/*                     ✨ COMPONENTS ✨                       */
/* ========================================================== */

const ANIMATION_DURATION: u64 = 200;

/// A toast element with the supplied alert style.
#[component]
pub fn Toast(toast: ToastData) -> impl IntoView {
    let slide_in_animation_name = get_slide_in_animation_name(&toast.position);
    let slide_out_animation_name = get_slide_out_animation_name(&toast.position);

    let animation_name_signal = RwSignal::new(slide_in_animation_name);

    let (background_color, border_color, text_color) = get_colors(&toast.level);
    let (initial_left, initial_right) = get_initial_positions(&toast.position);

    Effect::new(move |_| {
        if let Some(expiry) = toast.expiry {
            set_timeout(
                move || {
                    if !toast.clear_signal.get_untracked() {
                        let _ = toast.clear_signal.try_set(true);
                    }
                },
                Duration::from_millis(expiry as u64),
            );
        }
    });

    Effect::new(move |_| {
        let toaster = expect_toaster();

        if toast.clear_signal.get() {
            let _ = animation_name_signal.try_set(slide_out_animation_name);

            set_timeout(move || toaster.remove(toast.id), Duration::from_millis(ANIMATION_DURATION));
        }
    });

    let handle_click = move |_| {
        if !toast.dismissable {
            return;
        }

        toast.clear_signal.set(true);
    };

    view! {
        <style>
            {r#"
            @keyframes toast_tracker {
            0% { transform: scaleX(1); }
            100% { transform: scaleX(0); }
            }
            "#}
        </style>

        <div
            style:width="100%"
            style:margin="12px 0"
            style:padding="16px"
            style:background-color=background_color
            style:border="1px solid"
            style:border-color=border_color
            style:border-radius="4px"
            style:position="relative"
            style:cursor=get_cursor(toast.dismissable)
            style:overflow="hidden"
            style:box-sizing="border-box"
            style:left=initial_left
            style:right=initial_right
            style:display="flex"
            style:transition="transform 150ms ease-out, opacity 150ms ease-out"
            style:transition-delay="250ms, 0s"
            style:animation-name=animation_name_signal
            style:animation-duration=format!("{}ms", ANIMATION_DURATION)
            style:animation-timing-function="linear"
            style:animation-fill-mode="forwards"
            on:click=handle_click
        >
            <span
                style:color=text_color
                style:font-size="var(--leptoaster-font-size)"
                style:line-height="var(--leptoaster-line-height)"
                style:font-family="var(--leptoaster-font-family)"
                style:font-weight="var(--leptoaster-font-weight)"
                style:display="inline-block"
                style:max-width="100%"
                style:text-overflow="ellipsis"
                style:overflow="hidden"
            >
                {toast.message}
            </span>

            <Show when=move || { toast.expiry.is_some() && toast.progress }>
                <div
                    style:height="var(--leptoaster-progress-height)"
                    style:width="100%"
                    style:background-color=text_color
                    style:position="absolute"
                    style:bottom="0"
                    style:left="0"
                    style:transform-origin="left"
                    style:animation-name="toast_tracker"
                    style:animation-duration=format!("{}ms", toast.expiry.unwrap_or(0))
                    style:animation-timing-function="linear"
                    style:animation-fill-mode="forwards"
                />
            </Show>
        </div>
    }
}

const CONTAINER_POSITIONS: &[ToastPosition] = &[
    ToastPosition::TopLeft,
    ToastPosition::TopRight,
    ToastPosition::BottomRight,
    ToastPosition::BottomLeft,
];

#[component]
pub fn Toaster(#[prop(optional, into)] stacked: Signal<bool>) -> impl IntoView {
    let toaster = expect_toaster();

    view! {
        <style>{TEMPLATE_STYLES}</style>

        <For each=move || CONTAINER_POSITIONS key=|position| get_container_id(position) let:position>
            <Show when=move || !is_container_empty(position)>
                <div
                    class=get_container_class(stacked.get(), position)
                    style:width="var(--leptoaster-width)"
                    style:max-width="var(--leptoaster-max-width)"
                    style:margin=get_container_margin(position)
                    style:position="fixed"
                    style:inset=get_container_inset(position)
                    style:z-index="var(--leptoaster-z-index)"
                >
                    <For
                        each=move || {
                            let toasts = toaster.queue_signal.get();
                            match position {
                                ToastPosition::BottomLeft | ToastPosition::BottomRight => {
                                    toasts
                                        .iter()
                                        .filter(|toast| toast.position.eq(position))
                                        .cloned()
                                        .collect::<Vec<ToastData>>()
                                }
                                ToastPosition::TopLeft | ToastPosition::TopRight => {
                                    toasts
                                        .iter()
                                        .filter(|toast| toast.position.eq(position))
                                        .cloned()
                                        .rev()
                                        .collect::<Vec<ToastData>>()
                                }
                            }
                        }
                        key=|toast| toast.id
                        let:toast
                    >
                        <Toast toast=toast />
                    </For>
                </div>
            </Show>
        </For>
    }
}

/// A wrapper for showing toasts with a fluent API.
///
/// The wrapper only stores a `position` and an optional `level` override;
/// the actual message and default level are supplied when the toast method
/// (`success`, `error`, `info`, `warning`) is called.
pub struct ToastWrapper {
    level: Option<ToastLevel>,
    position: ToastPosition,
}

const DEFAULT_POSITION: ToastPosition = ToastPosition::BottomRight;

pub fn show_toast() -> ToastWrapper {
    ToastWrapper {
        level: None,
        position: DEFAULT_POSITION,
    }
}

impl ToastWrapper {
    fn emit(self, message: impl Into<String>, default_level: ToastLevel) {
        let toaster = expect_toaster();
        let level = self.level.unwrap_or(default_level);
        toaster.toast(
            ToastBuilder::new(message.into())
                .with_level(level)
                .with_position(self.position),
        );
    }

    pub fn success(self, message: impl Into<String>) {
        self.emit(message, ToastLevel::Success);
    }

    pub fn error(self, message: impl Into<String>) {
        self.emit(message, ToastLevel::Error);
    }

    pub fn info(self, message: impl Into<String>) {
        self.emit(message, ToastLevel::Info);
    }

    pub fn warning(self, message: impl Into<String>) {
        self.emit(message, ToastLevel::Warn);
    }

    /// Override the level (e.g. for custom levels).
    pub fn level(mut self, level: ToastLevel) -> Self {
        self.level = Some(level);
        self
    }

    /// Override the default position.
    pub fn position(mut self, position: ToastPosition) -> Self {
        self.position = position;
        self
    }
}

/* ========================================================== */
/*                     ✨ FUNCTIONS ✨                        */
/* ========================================================== */

pub fn provide_toaster() {
    if use_context::<ToasterContext>().is_none() {
        provide_context(ToasterContext::default());
    }
}

#[must_use]
pub fn expect_toaster() -> ToasterContext {
    expect_context::<ToasterContext>()
}

fn get_slide_in_animation_name(position: &ToastPosition) -> &'static str {
    match position {
        ToastPosition::TopLeft | ToastPosition::BottomLeft => "leptoaster-slide-in-left",
        ToastPosition::TopRight | ToastPosition::BottomRight => "leptoaster-slide-in-right",
    }
}

fn get_slide_out_animation_name(position: &ToastPosition) -> &'static str {
    match position {
        ToastPosition::TopLeft | ToastPosition::BottomLeft => "leptoaster-slide-out-left",
        ToastPosition::TopRight | ToastPosition::BottomRight => "leptoaster-slide-out-right",
    }
}

fn get_colors(level: &ToastLevel) -> (&'static str, &'static str, &'static str) {
    match level {
        ToastLevel::Info => (
            "var(--leptoaster-info-background-color)",
            "var(--leptoaster-info-border-color)",
            "var(--leptoaster-info-text-color)",
        ),
        ToastLevel::Success => (
            "var(--leptoaster-success-background-color)",
            "var(--leptoaster-success-border-color)",
            "var(--leptoaster-success-text-color)",
        ),
        ToastLevel::Warn => (
            "var(--leptoaster-warn-background-color)",
            "var(--leptoaster-warn-border-color)",
            "var(--leptoaster-warn-text-color)",
        ),
        ToastLevel::Error => (
            "var(--leptoaster-error-background-color)",
            "var(--leptoaster-error-border-color)",
            "var(--leptoaster-error-text-color)",
        ),
    }
}

fn get_initial_positions(position: &ToastPosition) -> (&'static str, &'static str) {
    match position {
        ToastPosition::TopLeft | ToastPosition::BottomLeft => (
            "calc((var(--leptoaster-width) + 12px * 2) * -1)",
            "auto",
        ),
        ToastPosition::TopRight | ToastPosition::BottomRight => (
            "auto",
            "calc((var(--leptoaster-width) + 12px * 2) * -1)",
        ),
    }
}

fn get_cursor(dismissable: bool) -> &'static str {
    match dismissable {
        true => "pointer",
        false => "default",
    }
}

fn is_container_empty(position: &ToastPosition) -> bool {
    !expect_toaster()
        .queue_signal
        .get()
        .iter()
        .any(|toast| toast.position.eq(position))
}

fn get_container_id(position: &ToastPosition) -> &'static str {
    match position {
        ToastPosition::TopLeft => "top_left",
        ToastPosition::TopRight => "top_right",
        ToastPosition::BottomRight => "bottom_right",
        ToastPosition::BottomLeft => "bottom_left",
    }
}

fn get_container_inset(position: &ToastPosition) -> &'static str {
    match position {
        ToastPosition::TopLeft => "0 auto auto 0",
        ToastPosition::TopRight => "0 0 auto auto",
        ToastPosition::BottomRight => "auto 0 0 auto",
        ToastPosition::BottomLeft => "auto 0 0 0",
    }
}

fn get_container_margin(position: &ToastPosition) -> &'static str {
    match position {
        ToastPosition::TopLeft | ToastPosition::BottomLeft => "0 0 0 12px",
        ToastPosition::TopRight | ToastPosition::BottomRight => "0 12px 0 0",
    }
}

fn get_container_class(stacked: bool, position: &ToastPosition) -> Option<&'static str> {
    if !stacked {
        return None;
    }

    match position {
        ToastPosition::BottomLeft | ToastPosition::BottomRight => {
            Some("leptoaster-stack-container-bottom")
        }
        ToastPosition::TopLeft | ToastPosition::TopRight => {
            Some("leptoaster-stack-container-top")
        }
    }
}