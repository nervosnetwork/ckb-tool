test: test-contract
	cargo test
test-contract:
	cd test-contract && capsule build

.PHONY: test test-contract

