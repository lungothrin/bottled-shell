PREFIX?=/opt/bottled-shell
TARGET=target/release/bottled-shell target/release/bottled
SOURCES=$(wildcard src/**/*.rs)
TEMPDIR=.build
PACKAGE=$(TEMPDIR)/installer.sh
CARGO?=cargo


.PHONY: all package install clean

all: $(TARGET)

package: $(PACKAGE)

$(TARGET): $(SOURCES)
	$(CARGO) build --release --locked --all-features

$(PACKAGE): $(TARGET)
	install -Dm555 target/release/bottled $(TEMPDIR)/prefix/bin/bottled && \
	tar --owner=root --group=root --mode=4555 -C $(TEMPDIR)/prefix \
		-cf $(TEMPDIR)/snapshot.tar bin/bottled && \
	install -Dm555 target/release/bottled-shell $(TEMPDIR)/prefix/bin/bottled-shell && \
	tar --owner=root --group=root -C $(TEMPDIR)/prefix \
		-uf $(TEMPDIR)/snapshot.tar bin/bottled-shell && \
	cat package/self-extracting/installer.sh $(TEMPDIR)/snapshot.tar >$(TEMPDIR)/installer.sh

install: $(PACKAGE)
	bash $(PACKAGE)

clean:
	$(RM) -R $(TEMPDIR)
	$(CARGO) clean
