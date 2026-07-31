# claude-account

[![Лицензия: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

[English version](README.md)

Переключатель аккаунтов Claude Code для macOS. Он создаёт отдельный
`CLAUDE_CONFIG_DIR` для каждого аккаунта и незаметно перенаправляет обычные
команды в официальный исполняемый файл Claude.

```bash
claude account add work
claude account add personal
claude account use work
claude account list
claude account current
claude account remove personal

claude
claude "fix this bug in main.py"
```

Вход, выход, хранение учётных данных и обновление токенов выполняет сам Claude
Code. `claude-account` не читает и не копирует содержимое учётных данных.

> [!IMPORTANT]
> Это независимый проект сообщества. Он не создан, не одобрен и не
> поддерживается Anthropic. Claude и Claude Code — продукты Anthropic.

## Требования

- macOS на Apple Silicon
- Установленный и работающий Claude Code
- Rust 1.85 или новее для сборки из исходного кода

## Установка из исходного кода

В каталоге проекта выполните:

```bash
cargo build --locked --release
./target/release/claude-account install
```

Установщик выведет строку вида `export PATH=...`. Добавьте её в `~/.zshrc`,
затем откройте новое окно терминала или выполните `source ~/.zshrc`. Shim
находится в отдельном каталоге и не заменяет официальный исполняемый файл
Claude.

Проверьте установку:

```bash
type -a claude
claude account list
```

Shim `claude-account` должен быть указан раньше официального Claude.

## Команды

### Добавить аккаунт

```bash
claude account add work
claude account add personal --email you@example.com
claude account add company --sso
claude account add api-billing --console
```

Команда открывает стандартный процесс входа Claude Code в изолированном
профиле. Первый добавленный профиль становится активным. Добавление следующего
профиля не переключает активный аккаунт. Команда также завершает локальную
настройку Claude Code, поэтому при следующем запуске `claude` повторный вход не
потребуется.

Параметры:

- `--email` — подставляет адрес электронной почты в форму входа.
- `--sso` — принудительно использует SSO-аутентификацию.
- `--console` — использует Anthropic Console вместо подписки Claude.

### Переключить аккаунт

```bash
claude account use work
```

Переключение действует на новые процессы Claude. Уже открытые сессии продолжают
работать с аккаунтом, с которым были запущены.

### Посмотреть профили

```bash
claude account list
claude account current
```

`list` выводит все зарегистрированные профили и помечает активный символом `*`.
`current` выводит только имя активного профиля, поэтому его удобно применять в
скриптах.

### Удалить аккаунт

```bash
claude account remove personal
```

Команда запускает официальный `auth logout` для профиля и убирает его из списка
аккаунтов. Настройки и история сессий сохраняются: если снова добавить профиль
с тем же именем, их можно использовать повторно.

Чтобы безвозвратно удалить все локальные данные профиля:

```bash
claude account remove personal --purge --yes
```

Активный профиль нельзя удалить без `--force`. Параметр `--purge` навсегда
удаляет его настройки, сессии, плагины и историю вместе с сохранённым входом.

### Справка

```bash
claude account --help
claude account add --help
claude account remove --help
```

Все команды и параметры, кроме `account`, без изменений передаются
официальному Claude Code:

```bash
claude
claude -p "explain this project"
claude --model opus
claude auth status --text
```

## Где хранятся данные

По умолчанию:

```text
~/Library/Application Support/claude-account/state.json
~/Library/Application Support/claude-account/profiles/<имя-профиля>/
~/Library/Application Support/claude-account/bin/claude
~/Library/Application Support/claude-account/libexec/claude-account
```

Необязательные переменные `XDG_CONFIG_HOME` и `XDG_DATA_HOME` поддерживаются.
`CLAUDE_ACCOUNT_HOME` позволяет разместить все данные приложения в одном
абсолютном каталоге — это особенно удобно для тестов.

В файле состояния содержатся имена профилей, пути к их каталогам и путь к
настоящему исполняемому файлу Claude. Токены доступа и обновления в нём не
хранятся.

## Переменные окружения для аутентификации

Чтобы выбранный профиль действительно использовался, wrapper удаляет из
дочернего процесса Claude следующие переменные:

- `ANTHROPIC_API_KEY`
- `ANTHROPIC_AUTH_TOKEN`
- `CLAUDE_CODE_OAUTH_TOKEN`

Установите `CLAUDE_ACCOUNT_PRESERVE_AUTH_ENV=1`, только если намеренно хотите,
чтобы эти переменные переопределяли аутентификацию профиля.

## Разработка

```bash
cargo fmt --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

Инструкции по внесению изменений — в [CONTRIBUTING.md](CONTRIBUTING.md),
информация о приватном сообщении уязвимостей — в [SECURITY.md](SECURITY.md).

## Лицензия

Проект распространяется по лицензии [MIT](LICENSE).
