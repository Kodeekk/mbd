BINARY  := mbd
CONFIG  := config.example.toml
SERVICE := mbd.service

USER_BINDIR  ?= $(HOME)/.local/bin
USER_SYSDDIR ?= $(HOME)/.config/systemd/user

SYSTEM_BINDIR  ?= /usr/local/bin
SYSTEM_SYSDDIR ?= /etc/systemd/system

.PHONY: all build install install-user install-system uninstall clean

all: build

build:
	cargo build --release --locked

install: install-user

install-user: build
	-systemctl --user stop mbd 2>/dev/null || true
	mkdir -p "$(USER_BINDIR)" "$(USER_SYSDDIR)" "$(HOME)/.config/mbd"
	install -m 755 target/release/$(BINARY) "$(USER_BINDIR)/$(BINARY)"
	sed "s|%h/.local/bin|$(USER_BINDIR)|g" $(SERVICE) > "$(USER_SYSDDIR)/$(SERVICE)"
	systemctl --user daemon-reload 2>/dev/null || true
	test -f "$(HOME)/.config/mbd/config.toml" || cp $(CONFIG) "$(HOME)/.config/mbd/config.toml"
	-systemctl --user enable --now mbd 2>/dev/null
	@echo "installed:  mbd status"

install-system:
	@if [ "$(shell id -u)" != "0" ]; then \
		echo "run with: sudo make install-system"; \
		exit 1; \
	fi
	@test -f target/release/$(BINARY) || { \
		echo "run 'make build' first, then: sudo make install-system"; \
		exit 1; \
	}
	mkdir -p "$(SYSTEM_BINDIR)" "$(SYSTEM_SYSDDIR)" /etc/mbd
	install -m 755 target/release/$(BINARY) "$(SYSTEM_BINDIR)/$(BINARY)"
	cp $(SERVICE) "$(SYSTEM_SYSDDIR)/$(SERVICE)"
	systemctl daemon-reload 2>/dev/null || true
	test -f /etc/mbd/config.toml || cp $(CONFIG) /etc/mbd/config.toml
	-systemctl enable --now mbd 2>/dev/null
	@echo "installed:  mbd status"

uninstall:
	-systemctl --user stop mbd 2>/dev/null || true
	-systemctl stop mbd 2>/dev/null || true
	rm -f "$(USER_BINDIR)/$(BINARY)" "$(USER_SYSDDIR)/$(SERVICE)"
	rm -f "$(SYSTEM_BINDIR)/$(BINARY)" "$(SYSTEM_SYSDDIR)/$(SERVICE)"
	systemctl --user daemon-reload 2>/dev/null || true
	systemctl daemon-reload 2>/dev/null || true

clean:
	cargo clean
