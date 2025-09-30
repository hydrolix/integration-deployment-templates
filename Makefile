all:
	@echo "targets are..."
	@echo "clean --> prunes docker and cargo"
	@echo "quick --> runs basic validation (no headless)"
	@echo "full --> runs with marketplace query limits with headless"

clean:
	cargo clean
	docker system prune -a -f
	
quick:
	cargo run

full:
	cargo run -- --local --marketplace

git-actions-locally:
	act -j bundle-validator --secret-file .secrets 
	

