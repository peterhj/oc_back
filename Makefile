CARGO := cargo --offline

.PHONY: all test debug rel release clean

all: debug

test:
	$(CARGO) test --lib --bins

debug:
	$(CARGO) build --lib --examples --bins
	mkdir -p debug_dist
	cp -p target/debug/oc_back_service debug_dist/
	strip debug_dist/oc_back_service
	mkdir -p ../debug_dist
	cp -p debug_dist/oc_back_service ../debug_dist/

rel: release

release:
	$(CARGO) build --release --lib --examples --bins
	mkdir -p dist
	cp -p target/release/oc_back_service dist/
	strip dist/oc_back_service
	mkdir -p ../dist
	cp -p dist/oc_back_service ../dist/

clean:
	rm -f dist/oc_back_service
	rm -rf target
