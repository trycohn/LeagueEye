# Инструкция: автообновление LeagueEye

## Первоначальная настройка сервера (один раз)

После `git pull` и пересборки Axum-сервера:

```bash
# Создать директорию для обновлений
mkdir -p /opt/leagueeye/updates

# Добавить переменную в .env сервера
echo 'UPDATES_DIR=/opt/leagueeye/updates' >> /path/to/server/.env

# Перезапустить сервер
```

Без файлов в этой директории сервер просто отвечает 204 (нет обновлений).

---

## Выпуск обновления

### 1. Обновить версию (два файла!)

**`src-tauri/tauri.conf.json`:**
```json
"version": "0.3.0"
```

**`src-tauri/Cargo.toml`:**
```toml
version = "0.3.0"
```

> Версии должны совпадать. Формат: `MAJOR.MINOR.PATCH` (semver).

### 2. Собрать с подписью (Windows, PowerShell)

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = "C:\Users\1337\.tauri\leagueeye.key"
npm run tauri build
```

После сборки в `target/release/bundle/nsis/` появятся:

| Файл | Назначение |
|---|---|
| `LeagueEye_0.3.0_x64-setup.exe` | Инсталлер + артефакт автообновления |
| `LeagueEye_0.3.0_x64-setup.exe.sig` | Подпись для проверки обновления |

> В Tauri v2 с `createUpdaterArtifacts: true` сам `.exe` инсталлер переиспользуется для обновлений (архив `.nsis.zip` не создаётся — это формат v1).

### 3. Загрузить на сервер

```powershell
scp target\release\bundle\nsis\LeagueEye_0.3.0_x64-setup.exe root@213.155.14.229:/opt/leagueeye/updates/
```

### 4. Создать latest.json на сервере

Открой `.exe.sig` файл — это одна строка текста (подпись). Скопируй её содержимое.

```bash
cat > /opt/leagueeye/updates/latest.json << 'EOF'
{
  "version": "0.3.0",
  "notes": "Что нового в этой версии",
  "pub_date": "2026-04-12T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "ВСТАВИТЬ_СОДЕРЖИМОЕ_ИЗ_.exe.sig_ФАЙЛА",
      "url": "http://213.155.14.229:3000/api/updates/download/LeagueEye_0.3.0_x64-setup.exe"
    }
  }
}
EOF
```

> Заменить `0.3.0` на актуальную версию, `notes` — описание обновления, `signature` — содержимое `.exe.sig` файла целиком.

### 5. Готово

Все пользователи с предыдущей версией:
- Автоматически увидят уведомление (проверка каждые 4 часа + при запуске)
- Могут вручную проверить в **Настройки → Обновления → Проверить обновления**
- Нажать **"Установить и перезапустить"** — приложение скачает, установит и перезапустится

---

## Где что лежит

| Что | Где | В git? |
|---|---|---|
| Приватный ключ подписи | `C:\Users\home\Documents\LeagueEye\~\.tauri\leagueeye.key` | НЕТ (в .gitignore) |
| Публичный ключ | `src-tauri/tauri.conf.json` → `plugins.updater.pubkey` | Да |
| Обновления на сервере | `/opt/leagueeye/updates/` | Нет (на сервере) |
| Env переменная сервера | `UPDATES_DIR=/opt/leagueeye/updates` | Нет (.env) |

---

## Как работает автообновление

```
Приложение (каждые 4 часа + при запуске)
  → GET http://213.155.14.229:3000/api/updates/windows/x86_64/0.2.0
  
Сервер читает /opt/leagueeye/updates/latest.json
  → Сравнивает версии (semver)
  → Если новее — отдаёт JSON с url и signature
  → Если нет — 204 No Content

Приложение (при нажатии "Установить")
  → GET http://213.155.14.229:3000/api/updates/download/LeagueEye_0.3.0_x64-setup.exe
  → Проверяет подпись публичным ключом
  → Устанавливает (NSIS passive mode)
  → Перезапускается
```

---

## Быстрый чеклист релиза

- [ ] Обновить версию в `tauri.conf.json` и `Cargo.toml`
- [ ] Собрать: `npm run tauri build` (с `TAURI_SIGNING_PRIVATE_KEY_PATH`)
- [ ] Загрузить `.exe` на сервер в `/opt/leagueeye/updates/`
- [ ] Обновить `latest.json` (version, signature, url, notes)
- [ ] Проверить: открыть приложение старой версии → Настройки → Проверить обновления

---

## Устранение проблем

**Обновление не находится:**
- Проверь что `latest.json` корректный JSON: `cat /opt/leagueeye/updates/latest.json | python3 -m json.tool`
- Проверь что версия в `latest.json` больше текущей версии приложения
- Проверь endpoint: `curl http://213.155.14.229:3000/api/updates/windows/x86_64/0.2.0`

**Ошибка подписи:**
- Убедись что `.exe.sig` файл был создан тем же приватным ключом, чей публичный ключ в `tauri.conf.json`
- Скопируй содержимое `.exe.sig` файла полностью, без лишних пробелов/переносов

**Потерян приватный ключ:**
- Обновления для текущих пользователей станут невозможны
- Нужно сгенерировать новую пару ключей и выпустить полный инсталлер (не обновление)
