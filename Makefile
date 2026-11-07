BIN := target/release/eselect

.PHONY: build install uninstall man completions test clean deltainst

build:
	cargo build --release

test:
	cargo test --release

man: build
	rm -rf man && mkdir -p man
	$(BIN) --gen-manpage man
	sudo install -Dm644 man/*.1 -t /usr/share/man/man1/

completions: build
	$(BIN) --gen-completions bash | sudo tee /usr/share/bash-completion/completions/eselect > /dev/null
	$(BIN) --gen-completions zsh | sudo tee /usr/share/zsh/site-functions/_eselect > /dev/null
	$(BIN) --gen-completions fish | sudo tee /usr/share/fish/vendor_completions.d/eselect.fish > /dev/null

# A real /usr/bin/eselect replaces the AE shim (/usr/local/bin/eselect).
install: build man completions
	sudo install -Dm755 $(BIN) /usr/bin/eselect
	@if [ -x /usr/lib/aura-emerge/eselect-shim ]; then sudo /usr/lib/aura-emerge/eselect-shim remove; fi

uninstall:
	sudo rm -f /usr/bin/eselect /usr/share/bash-completion/completions/eselect \
		/usr/share/zsh/site-functions/_eselect /usr/share/fish/vendor_completions.d/eselect.fish
	sudo rm -f /usr/share/man/man1/eselect.1 /usr/share/man/man1/eselect-*.1
	@if [ -x /usr/lib/aura-emerge/eselect-shim ]; then sudo /usr/lib/aura-emerge/eselect-shim install; fi

clean:
	cargo clean
	rm -rf man comp

deltainst:
	RUSTFLAGS="-C target-cpu=znver3 -C opt-level=3 -C codegen-units=1 -C target-feature=+avx2,+fma,+bmi1,+bmi2,+aes,+sha,+sse4.2,+popcnt,+vaes,+vpclmulqdq,+movbe,+f16c" cargo build --release
	$(MAKE) man completions
	sudo install -Dm755 $(BIN) /usr/bin/eselect
	@if [ -x /usr/lib/aura-emerge/eselect-shim ]; then sudo /usr/lib/aura-emerge/eselect-shim remove; fi
