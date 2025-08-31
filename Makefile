clean:
	cargo clean
	docker system prune -a -f
	
run-locally:
	cargo run --package bundle-validator

test-locally:
	act --container-architecture linux/amd64 -P urust:latest



