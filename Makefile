CARGO := cargo --offline

.PHONY: all test clean

all:
	$(CARGO) build --lib --examples --bins
	mkdir -p debug_dist
	cp -p target/debug/oc_back_service debug_dist/
	strip debug_dist/oc_back_service

test:
	$(CARGO) test --lib --bins

clean:
	rm -f dist/oc_back_service
	rm -rf target
