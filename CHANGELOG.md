# Changelog

Все значимые изменения в этом проекте документируются в этом файле.

Формат основан на [Keep a Changelog](https://keepachangelog.com/ru/1.0.0/),
а проект придерживается [Семантического Версионирования](https://semver.org/lang/ru/).

## [Unreleased]

### Refactored
- **shared**: реорганизованы модели `tenant` и `user` в директорию `models/` для улучшения архитектуры домена и соответствия строгим требованиям документирования (`#![deny(missing_docs)]`).

### Docs
- **specs**: актуализированы `STRUCTURE.md` и `STATUS.md` после реорганизации моделей в крейте `shared`.