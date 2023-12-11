CARGO := cargo --offline

.PHONY: all test clean

all:
	$(CARGO) build --lib --bins

test:
	$(CARGO) test --lib --bins

clean:
	rm -rf target
