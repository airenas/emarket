-include Makefile.options
###############################################################################
run:
	RUST_LOG=DEBUG cargo run --bin importer -- --key $(KEY) --redis $(REDIS_URL)
.PHONY: run
###############################################################################
run/ws:
	RUST_LOG=DEBUG cargo run --bin importer-ws -- --redis $(REDIS_URL)
.PHONY: run/ws
###############################################################################
run/build: build/local
	RUST_LOG=DEBUG target/release/importer --key $(KEY) --redis $(REDIS_URL)
.PHONY: run/build
###############################################################################
build/local: 
	cargo build --release
.PHONY: build/local
###############################################################################
test/unit:
	RUST_LOG=DEBUG cargo test --no-fail-fast
.PHONY: test/unit		
test/lint:
	@cargo clippy -V
	cargo clippy --all-targets --all-features -- -D warnings
.PHONY: test/lint	
###############################################################################
clean:
	rm -r -f target
.PHONY: clean
