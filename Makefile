clean:
	cargo clean
	docker system prune -a -f
	
run-locally:
	cargo run --package bundle-validator

test-locally:
	#act -j bundle-validator -P javiani/my-act-image:latest  --secret-file .secrets --container-architecture linux/amd64

	act -j bundle-validator --secret-file .secrets 
	

