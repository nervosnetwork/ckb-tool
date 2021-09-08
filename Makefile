test: test-contract
	cargo test
test-contract:
	cd test-contract && capsule build && capsule test

.PHONY: test test-contract

