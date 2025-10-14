all:
	@echo "targets are..."
	@echo "clean --> prunes docker and cargo"
	@echo "quick --> runs basic validation (no headless)"
	@echo "full --> runs with marketplace query limits with headless"

clean:
	cargo clean
	docker system prune -a -f
	
super-clean:
	@echo "Performing super-clean..."
	rm -rf Cargo.lock
	rm -rf ~/.cargo/registry
	rm -rf ~/.cargo/git
	rm -rf target
	docker system prune -a -f

quick:
	cargo run

full:
	cargo run -- --local --marketplace

git-actions-locally:
	act -j bundle-validator --secret-file .secrets 
	
audit:
	cargo install cargo-audit
	cargo audit 

coding-standards:
	@echo "\n=== Format Code ==="
	cargo fmt

	@echo "\n=== Checking Clippy ==="
	cargo clippy -- \
		-W clippy::unwrap_used \
		-W clippy::expect_used \
		-W clippy::panic \
		-W clippy::todo \
		-W clippy::unimplemented \
		-W clippy::unreachable \
		-A clippy::question_mark
	
	@echo "\n=== Checking for ? operator usage ==="
	@if grep -rE '\?\s*($$|;)' --include="*.rs" src/ 2>/dev/null; then \
		echo "❌ ERROR: Found ? operator usage"; \
		exit 1; \
	else \
		echo "✅ No ? operator found"; \
	fi