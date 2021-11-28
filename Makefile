PREFIX?=/opt/bottled-shell
CARGO?=cargo
TARGET=target/release/bottled-shell target/release/bottled
SOURCES=$(wildcard src/**/*.rs)


.PHONY: all install clean

all: $(TARGET)

$(TARGET): $(SOURCES)
	$(CARGO) build --release --locked --all-features

install: $(TARGET)
	install -Dm4555 target/release/bottled "${PREFIX}/bin/bottled"
	install  -Dm555 target/release/bottled-shell "${PREFIX}/bin/bottled-shell"

clean:
	$(CARGO) clean
