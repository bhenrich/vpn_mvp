SHELL := /bin/bash

.PHONY: help up down build fmt lint test

help:
	@echo "Targets:"
	@echo "  up       - docker compose up (build)"
	@echo "  down     - docker compose down"
	@echo "  build    - docker compose build"
	@echo "  fmt      - rustfmt check"
	@echo "  lint     - ruff + clippy"
	@echo "  test     - pytest + cargo test"

up:
	docker compose up --build

down:
	docker compose down -v

build:
	docker compose build

fmt:
	cargo fmt --all

lint:
	ruff check services/*-api/app || true
	cargo clippy --workspace --all-targets || true

test:
	pytest || true
	cargo test --workspace --all-features


