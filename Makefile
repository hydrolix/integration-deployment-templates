clean:
	cargo clean
	docker system prune -a -f
	
run-local-marketplace:
	cargo run --package bundle-validator -- --local --marketplace

git-actions-locally:
	act -j bundle-validator --secret-file .secrets 
	

