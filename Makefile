# ==============================================================================
# Player Scout — Makefile de Automação de Tarefas de Desenvolvimento
# ==============================================================================

.PHONY: help dev run seed check fmt test test-unit test-integration docker-up docker-down docker-ps swagger clean

help:
	@echo "🎾 Player Scout — Comandos Disponíveis:"
	@echo ""
	@echo "  make dev                Sobe o PostgreSQL 18 e inicia o servidor backend Axum"
	@echo "  make run                Inicia apenas o servidor backend Axum (porta 3000)"
	@echo "  make seed               Popula o banco local com treinador, 4 atletas e partidas de demonstração"
	@echo "  make check              Executa cargo fmt --check e cargo clippy (zero warnings)"
	@echo "  make fmt                Aplica a formatação padrão oficial do Rust (cargo fmt)"
	@echo "  make test               Executa 100% dos testes automatizados (unitários + integração)"
	@echo "  make test-unit          Executa apenas os testes rápidos de domínio (sem banco)"
	@echo "  make test-integration   Executa os testes de integração com o PostgreSQL 18"
	@echo "  make docker-up          Sobe os containers do PostgreSQL 18 e MCP em segundo plano"
	@echo "  make docker-down        Para os containers do Docker Compose"
	@echo "  make docker-ps          Exibe o status e healthcheck dos containers"
	@echo "  make swagger            Exibe as URLs da documentação interativa OpenAPI 3.1"
	@echo "  make clean              Limpa os artefatos de compilação da pasta target/"
	@echo ""

# Iniciar o ambiente de desenvolvimento completo
dev: docker-up run

# Executar a aplicação Axum
run:
	cargo run --manifest-path rust/Cargo.toml

# Executar o seed de demonstração
seed:
	cargo run --bin seed --manifest-path rust/Cargo.toml

# Verificação rigorosa de qualidade (CI local)
check:
	cargo fmt --check --manifest-path rust/Cargo.toml
	cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings

# Formatação automática
fmt:
	cargo fmt --manifest-path rust/Cargo.toml

# Executar todos os testes
test: test-unit test-integration

# Testes unitários de domínio puro
test-unit:
	cargo test --manifest-path rust/Cargo.toml --test domain_validation_tests

# Testes de integração com PostgreSQL
test-integration:
	cargo test --manifest-path rust/Cargo.toml --test api_integration_tests

# Comandos Docker Compose
docker-up:
	docker compose up -d

docker-down:
	docker compose down

docker-ps:
	docker compose ps

# Informações do Swagger UI
swagger:
	@echo "📖 Swagger UI:  http://localhost:3000/swagger-ui/"
	@echo "📄 OpenAPI JSON: http://localhost:3000/api-docs/openapi.json"

clean:
	cargo clean --manifest-path rust/Cargo.toml
