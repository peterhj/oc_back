CARGO := cargo --offline

.PHONY: all debug test clean

all: debug

debug:
	$(CARGO) build --lib --examples --bins
	mkdir -p debug_dist
	cp -p target/debug/oc_back_service debug_dist/
	strip debug_dist/oc_back_service
	mkdir -p ../debug_dist
	cp -p debug_dist/oc_back_service ../debug_dist/

test:
	$(CARGO) test --lib --bins

clean:
	rm -f dist/oc_back_service
	rm -rf target
