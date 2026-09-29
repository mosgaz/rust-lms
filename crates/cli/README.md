# rust-kit

CLI-утилита для генерации Tailwind CSS v4 темы в Rust/Leptos проектах.

## Что это

`rust-kit` генерирует два CSS-файла в вашем проекте:

- `assets/styles/tailwind.css` — статический шаблон с `@import`, `@theme inline` и `@layer base`.
- `assets/styles/colors.css` — CSS-переменные (`:root` и `.dark`) для выбранной темы.

Тема выбирается интерактивно (или через флаг `--yes` для дефолтов). Никаких `package.json`, `npm`, `node_modules` — только Rust и Tailwind CSS.

## Установка

```bash
cargo install --git https://github.com/your/rust-kit rust-kit-cli
```

## Использование

### Интерактивно

```bash
$ rust-kit init
? Base color: › Neutral
? Accent color: › Default
✅ assets/styles/tailwind.css written.
✅ assets/styles/colors.css written.
```

### С дефолтами

```bash
$ rust-kit init --yes
✅ assets/styles/tailwind.css written.
✅ assets/styles/colors.css written.
```

### Перезапись существующих файлов

```bash
$ rust-kit init --force
✅ assets/styles/tailwind.css written.
✅ assets/styles/colors.css written.
```

Без `--force` и без `--yes` CLI спросит подтверждение, если файлы уже существуют:

```bash
$ rust-kit init
? assets/styles/tailwind.css already exists. Overwrite? (y/N) › n
⏭️  Skipping assets/styles/tailwind.css
? assets/styles/colors.css already exists. Overwrite? (y/N) › y
✅ assets/styles/colors.css written.
```

## Доступные темы

### Base colors

`Neutral`, `Stone`, `Zinc`, `Mauve`, `Olive`, `Mist`, `Taupe`

### Accent colors

`Default`, `Amber`, `Blue`, `Cyan`, `Emerald`, `Fuchsia`, `Green`, `Indigo`, `Lime`, `Orange`, `Pink`, `Purple`, `Red`, `Rose`, `Sky`, `Teal`, `Violet`, `Yellow`

## Что генерируется

После `rust-kit init` в вашем проекте появляется:

```
your-project/
├── assets/
│   └── styles/
│       ├── tailwind.css     # статический шаблон
│       └── colors.css       # сгенерированные CSS-переменные
├── Cargo.toml
└── src/
```

### `tailwind.css`

```css
@import "tailwindcss";
@import "tw-animate-css";
@import "./colors.css";

@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  /* ... */
}

@layer base {
  * { @apply border-border outline-ring/50; }
  body { @apply bg-background text-foreground; }
  /* ... */
}
```

### `colors.css`

```css
:root {
  --radius: 0.625rem;
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
  /* ... */
  --destructive: oklch(0.577 0.245 27.325);
}

.dark {
  --background: oklch(0.145 0 0);
  --foreground: oklch(0.985 0 0);
  /* ... */
  --destructive: oklch(0.704 0.191 22.216);
}
```

## Использование в Leptos-проекте

Подключите `assets/styles/tailwind.css` как input для Tailwind CLI:

```bash
tailwindcss -i assets/styles/tailwind.css -o assets/styles/output.css --watch
```

Или через `Trunk.toml`:

```toml
[build]
tailwindcss = "assets/styles/tailwind.css"
```

Tailwind сам разберёт `@import "./colors.css"` и подтянет переменные.

## Структура репозитория

```
rust-kit/
├── Cargo.toml
├── README.md
└── crates/
    └── cli/
        ├── Cargo.toml
        └── src/
            ├── main.rs
            ├── cli.rs            # clap-команды
            ├── error.rs
            ├── prompts.rs        # dialoguer-промпты
            ├── templates.rs      # статический tailwind.css
            ├── theme/
            │   ├── mod.rs
            │   └── colors.rs     # BaseColor, AccentColor, generate_theme_vars
            └── commands/
                ├── mod.rs
                └── init.rs       # логика init
```

## Разработка

### Требования

- Rust (edition 2024, stable toolchain)
- Cargo

### Сборка

```bash
cargo build
```

### Запуск CLI локально

```bash
cargo run -p rust-kit-cli -- init
cargo run -p rust-kit-cli -- init --yes
cargo run -p rust-kit-cli -- init --force
```

### Тестирование

```bash
cd crates/cli
cargo test
```

Или из корня:

```bash
cargo test -p rust-kit-cli
```

С выводом `println!`:

```bash
cargo test -p rust-kit-cli -- --nocapture
```

### Линт и форматирование

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

### Проверка сборки

```bash
cargo check --workspace
```

## Что есть

- ✅ Генерация `tailwind.css` (статический шаблон).
- ✅ Генерация `colors.css` (зависит от `base` и `accent`).
- ✅ Интерактивный выбор темы через `dialoguer`.
- ✅ Флаги `--yes` (дефолты) и `--force` (перезапись без вопросов).
- ✅ Подтверждение перед перезаписью существующих файлов.
- ✅ Пропуск перезаписи, если содержимое идентично.
- ✅ 7 base colors, 18 accent colors.
- ✅ Чистый Rust, без `package.json`, `npm`, `node_modules`.
- ✅ Unit-тесты для `theme::colors` и `commands::init`.

## Чего нет (планируется)

- ⏳ Проверка `Cargo.toml` целевого проекта (наличие `leptos`, `tailwindcss` и т.д.).
- ⏳ Команда `rust-kit theme set --base <color> --accent <color>` для перегенерации только `colors.css`.
- ⏳ Команда `rust-kit theme list` для просмотра доступных тем.
- ⏳ Управление компонентами (`rust-kit add <component>`).

## Лицензия

MIT OR Apache-2.0