.PHONY: build release run test lint fmt fmt-check typecheck checkall hooks install clean deploy ci gallery grind-start grind-start-anthropic grind-start-zai grind-start-grok grind-start-codex grind-start-omp grind-stop grind-clean-logs

build:
	cargo build

release:
	cargo build --release

run:
	cargo run --release -- $(ARGS)

test:
	cargo test

lint:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

typecheck:
	cargo check

checkall: fmt-check lint typecheck test build

# Point git at the repo's hooks dir so .githooks/pre-commit actually runs.
hooks:
	git config core.hooksPath .githooks
	@echo "core.hooksPath=.githooks — pre-commit now runs make checkall"

install:
	cargo install --path .

# Capture all animations to ./gallery as PNG+GIF with index.html.
# Filter or override via ARGS, e.g.:
#   make gallery
#   make gallery ARGS="fire,plasma"
#   make gallery ARGS="fire --gallery-dir /tmp/g"
gallery:
	cargo run --release -- --gallery $(ARGS)

clean:
	cargo clean

# Trigger CI workflow on GitHub
ci:
	gh workflow run ci.yml
	@echo "CI triggered — watch at https://github.com/paulrobello/termflix/actions"

# Trigger release + deploy workflow on GitHub
deploy:
	gh workflow run release.yml
	@echo "Deploy triggered — watch at https://github.com/paulrobello/termflix/actions"

# ==== Grind loop (~/Repos/par-grind) ========================================
# Example Makefile fragment: the full grind target set, ready to paste into a
# project Makefile. Targets are project-agnostic — per-project knobs (prompt,
# model, clean command, post-iter check) live in the project's
# .grind.local.json; every GRIND_* env var overrides a config key. See the
# README's config table.
#
# `include`-ing this file also works (targets resolve exactly the same), but a
# naive help target that greps `## ` out of $(MAKEFILE_LIST) renders each
# included-file line file-prefixed (BSD and GNU grep both prefix matches when
# scanning multiple files). Paste if you want clean help output; include if you
# want to track this repo.
#
# Invoked via `bash` on purpose: `ps` then shows `bash …/grind.sh <repo>`,
# the command shape grind's overseer recipes and probes match on. A
# shebang-direct invocation puts the script at a different ps field and reads
# as grind GONE to them.
GRIND_SH := $(HOME)/Repos/par-grind/grind.sh

# grind-start runs the z.ai backend at effort high (2026-09-12);
# grind-start-zai keeps the zai arm's default effort (xhigh).
grind-start: ## Run the grind loop against this repo (foreground; z.ai backend, effort high)
	GRIND_BACKEND=zai GRIND_EFFORT=high bash $(GRIND_SH) "$(CURDIR)"

grind-start-anthropic: ## Run the grind loop against the Anthropic backend (foreground; model sonnet[1m])
	GRIND_BACKEND=anthropic GRIND_MODEL=sonnet[1m] bash $(GRIND_SH) "$(CURDIR)"

grind-start-zai: ## Run the grind loop against the z.ai backend (foreground; needs ZAI_API_KEY)
	GRIND_BACKEND=zai bash $(GRIND_SH) "$(CURDIR)"

grind-start-grok: ## Run the grind loop against the Grok backend (foreground; needs grok CLI login)
	GRIND_BACKEND=grok bash $(GRIND_SH) "$(CURDIR)"

grind-start-codex: ## Run the grind loop against the Codex backend (foreground; needs codex CLI login)
	GRIND_BACKEND=codex bash $(GRIND_SH) "$(CURDIR)"

# The omp arm is the only one with NO default model: OMP v18 applies
# --provider during explicit model resolution, so grind.sh derives omp's
# --provider from the exact model id (glm-5.3/glm-5.3-flash -> zai,
# gpt-5.6-terra/gpt-5.6-luna -> openai-codex, grok-4.7 -> xai-oauth).
# Refuse to launch without one rather than guess a provider.
grind-start-omp: ## Run the grind loop against the OMP backend (foreground; GRIND_MODEL required — provider derived from the exact model id)
	@test -n "$(GRIND_MODEL)" || { echo "grind-start-omp: GRIND_MODEL is required, e.g. GRIND_MODEL=glm-5.3 — the omp arm derives omp's --provider from the exact model id (glm-5.3/glm-5.3-flash->zai, gpt-5.6-terra/gpt-5.6-luna->openai-codex, grok-4.7->xai-oauth)" >&2; exit 2; }
	GRIND_BACKEND=omp GRIND_MODEL="$(GRIND_MODEL)" bash $(GRIND_SH) "$(CURDIR)"

grind-stop: ## Ask a running grind loop to stop after the current iteration
	@touch .exit-grind
	@echo "wrote .exit-grind — the loop stops after the current iteration finishes"

# .exit-grind is checked only at the top of each iteration: a stop takes
# effect once the in-flight session ends, and grind removes the file itself
# on exit — a grind-stop with nothing running just clears a stale sentinel.
# grind-start runs in the FOREGROUND (it owns the terminal); grind-stop runs
# from a second shell.

# 2*/ matches only the date-named run dirs — the latest symlink, the run.jsonl
# ledger, and stdout logs live at the root and never match. The newest 2 by
# mtime are kept (the active run is always newest, and the latest symlink's
# own target is excluded from removal as a belt-and-braces guard).
grind-clean-logs: ## Delete all but the newest 2 grind run dirs (latest symlink, run.jsonl kept)
	@latest=$$(readlink .grind-logs/latest 2>/dev/null | xargs basename 2>/dev/null); \
	old=$$(ls -dt .grind-logs/2*/ 2>/dev/null | tail -n +3 | sed 's:/$$::'); \
	[ -n "$$old" ] || { echo "grind-clean-logs: nothing to remove (<= 2 run dirs)"; exit 0; }; \
	if [ -n "$$latest" ]; then old=$$(echo "$$old" | grep -v "/$$latest$$"); fi; \
	[ -n "$$old" ] || { echo "grind-clean-logs: nothing to remove"; exit 0; }; \
	echo "$$old" | sed 's/^/  removing /'; \
	echo "$$old" | xargs rm -rf; \
	echo "grind-clean-logs: $$(ls -d .grind-logs/2*/ 2>/dev/null | wc -l | tr -d ' ') run dir(s) remain"
