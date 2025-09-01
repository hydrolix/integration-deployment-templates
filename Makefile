clean:
	cargo clean
	docker system prune -a -f
	
run-locally:
	cargo run --package bundle-validator

test-locally:
	#act -j bundle-validator  no-pull --container-architecture linux/amd64 -P my-act-image:latest
	# The mapping should be: runner-name=image-name
	act -j bundle-validator --container-architecture linux/amd64 -P javiani/my-act-image:latest  --secret-file .secrets   --container-options "--network=host"

	# act -j bundle-validator  no-pull --container-architecture linux/amd64 -P ubuntu-latest=catthehacker/ubuntu:act-latest

fucker-test-locally:
	docker pull rust:latest
	act --container-architecture linux/amd64 -P rust:latest

