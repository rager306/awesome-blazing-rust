# Upstream daily scan: strike ledger, breaker, daily links — Implementation Plan

Goal: замкнуть цикл upstream-проверки abr — daily-расписание, страйк-леджер с цепным тормозом 3%, no-op гейты, дедуп issue; enforcement (удаление после 3 страйков) — фаза 2 после калибровки.

Approved direction: brainstorm 2026-10-27 (все три решения — как в рекомендациях; каденс исправлен пользователем на daily).
Acceptance criteria:
- `abr upstream` локально и в CI пишет/обновляет `data/upstream-state.json`, печатает отчёт, карточки не трогает.
- >3% Unreachable за прогон → state заморожен, отчёт «incident», мутации нет.
- Повторный прогон без изменений → проекция и state бит-в-бит, коммита нет.
- links.yml: daily cron; lychee-отчёт дедуплицируется (обновление существующего issue, не новый спам); upstream-шаг пушит в main только при диффе; CI зелёный после пуша.
- (Фаза 2, отдельный план) 3 страйка → удаление карточки + catalog/removed.json.

Out of scope:
- Удаление карточек (фаза 2), resurrect-страница/автовозврат, GraphQL-батчи, SEO-слой, проверка не-GitHub ссылок внутри abr (lychee уже покрывает их репортом).

## Project Context

Relevant files:
- `cli/src/domain/upstream.rs`: pure-ядро — repo_slug, проекция json(), parse_prev(), diff(). Сюда добавить страйк-логику (pure, TDD).
- `cli/src/ports/upstream.rs`: UpstreamSource trait + TagState (Tag/NoTags/Unreachable). Уже различает легитимное NoTags от Unreachable — переиспользуем.
- `cli/src/app/mod.rs`: run_upstream() (строки 75+) — оркестрация; сюда врезать state/breaker.
- `cli/src/adapters/gh_curl.rs`: GhCli через subprocess `gh api` — не трогаем.
- `cli/src/adapters/fs_io.rs` / `ports/sink.rs`: Source+Sink для read_opt/write — через них state.
- `data/upstream.json`: НЕ существует — проекция ни разу не коммитилась (цикл не замкнут).
- `.github/workflows/links.yml`: weekly lychee + issue; станет daily + upstream-шаг + дедуп.
- `.github/workflows/lint.yml`: гоняет `cargo test` на каждый push — тесты фазы 1 исполняются в CI автоматически.

Existing patterns to follow:
- Zero-dependency ethos (subprocess gh, без крейтов).
- Pure domain / I/O в адаптерах; FSM Boot→Loaded→Checked; детерминированные проекции (сортировка, формат — контракт парсера).
- read_opt для опциональных файлов (upstream.json уже так читается).

## Task 1: Strike ledger + breaker в pure domain (TDD)

Objective: чистые функции страйков и цепного тормоза.
Files:
- modify/test: `cli/src/domain/upstream.rs` (тесты в `#[cfg(test)]` там же — существующий стиль).
Steps:
1. Тесты на новую структуру `Strike` (BTreeMap<String,u32>) и функции:
   - `apply(strike, outcomes: &[(repo, TagState)]) -> Strike`: Unreachable → +1; Tag/NoTags → сброс в 0; новый repo с успехом → 0 (запись).
   - `incident(outcomes) -> bool`: доля Unreachable > 3% (строго больше; 0 из 0 → false).
   - `tripwire(strike) -> Vec<String>`: repos с consecutive_failures >= 3 (для фазы 2 и отчёта).
2. Run: `cd cli && cargo test upstream` — Expected: новые тесты красные (функций нет).
3. Минимальная реализация.
4. Run: `cargo test` — Expected: все зелёные, включая старые.
5. Checkpoint: коммит `upstream: strike ledger + 3% incident breaker (pure domain)`.

## Task 2: State в run_upstream (wiring)

Objective: run_upstream читает/пишет `data/upstream-state.json`, применяет breaker.
Files:
- modify: `cli/src/app/mod.rs` (run_upstream), возможно `cli/src/domain/fsm.rs` (если UPSTREAM-константа пути там).
Steps:
1. Читать `data/upstream-state.json` через io.read_opt (формат: `{"repo": N}` построчно — тот же стиль контракта, что upstream.json; парсер в domain).
2. После сбора rows/missing: `outcomes` → incident? → да: НЕ писать state, в summary добавить `incident: {missing}/{total} unreachable (> 3%), state frozen`; нет: apply + write через io.
3. Отчёт: секции `new strikes` (инкрементированные), `recovered` (сброшенные), `tripwire` (>= 3, упоминание «enforcement phase 2»).
4. Run: `cd cli && cargo test && cargo build --release`.
5. Run: `GITHUB_TOKEN=$(gh auth token) ./target/release/abr upstream` — Expected: `data/upstream.json` и `data/upstream-state.json` созданы, summary перечисляет changed/gone/strikes.
6. Run: повторно — Expected: state бит-в-бит (или только last-run независимые поля — их нет в схеме), диффа нет.
7. Checkpoint: коммит `upstream: persist strike state with incident freeze`.

## Task 3: links.yml → daily + upstream step + issue dedup

Objective: ежедневный прогон; upstream-проекция пушится при диффе; lychee-issue не плодится.
Files:
- modify: `.github/workflows/links.yml`.
Steps:
1. cron → `17 6 * * *` (06:17 UTC); permissions добавить `contents: write`.
2. Новый job `upstream` (до lychee): checkout с `token: github.token` (push), dtolnay/rust-toolchain@stable, build, `abr upstream` с env GITHUB_TOKEN: github.token; затем: если `git status --porcelain data/` не пуст → `git commit -m "Upstream scan $(date -u +%F)" && git push`. Пусто → echo "No changes."
3. Lychee job: `needs: upstream`; дедуп issue вместо безусловного create: шаг `gh issue list --label link-rot --state open --json number -q '.[0].number'`; есть → `gh issue comment N --body-file lychee/out.md` (или edit body); нет → create-issue-from-file как сейчас.
4. Run: `workflow_dispatch` из UI/CLI. Expected: upstream job закоммитил data/upstream.json + state; lychee job отработал; issue создан/обновлён только при реальных битых; CI (Lint) зелёный после пуша.
5. Повторный dispatch на след. день: Expected: «No changes» в upstream job, новых issue нет.
6. Checkpoint: коммит `links: daily cadence, upstream scan step, issue dedup`.

## Task 4: Review and verification

Objective: сверить acceptance criteria фазы 1.
Steps:
1. `gh run view <id> --json jobs` — оба jobs success.
2. `git log --oneline -3` — ровно два новых коммита (upstream scan, links workflow), без пустых.
3. Локальный повтор `abr upstream` → «No changes»/стабильный state (детерминизм проекции).
4. Ручная симуляция breaker: временно подменить GhCli результат нельзя в CI — проверяется юнит-тестом из Task 1 (>3% → incident true), в отчёте живого прогона поле отсутствует.
5. Итоговый отчёт пользователю: что появилось в репо, как выглядит отчёт, что будет в фазе 2.

## Фаза 2 (отдельный план после 2–4 дневных прогонов)
- `abr upstream --enforce`: tripwire (>= 3 страйков) → удаление карточки из README + regen patterns/catalog/llms + `catalog/removed.json` (url, name, removed_at, last_status, tag-at-removal); breaker 3% и на удаления; resurrect-проверка (repо из removed.json снова жив → предложить re-add).
- Требует Sink-запись README: markdown.rs парсит, но обратной записи нет — оценить объём до старта.
