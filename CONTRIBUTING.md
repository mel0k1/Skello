# Участие в разработке

## Сборка и проверки

Нужны Rust stable и Flutter stable. Перед коммитом прогоняйте:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cd app
flutter analyze
flutter test
```

CI гоняет то же самое на Linux и Windows, незелёный PR не вливаем.

## Стиль

- Комментарии в коде объясняют «почему», а не повторяют строку кода
- Новые протокольные факты (константы, форматы) сверяем с исходниками AOSP и помечаем, откуда взяты
- Тексты интерфейса - на русском, термины adb/fastboot оставляем как есть
- Коммиты в повелительном наклонении: «add fastboot driver», а не «добавил»
