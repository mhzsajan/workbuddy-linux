SHELL := /bin/bash

.PHONY: help deps preflight build-app run-app deb rpm pacman package install clean check release-bundle release release-local

help:
	@echo "Build:"
	@echo "  make deps"
	@echo "  make preflight"
	@echo "  make build-app"
	@echo "  make build-app DMG=/path/to/WorkBuddy.dmg"
	@echo "  make build-app DMG=/path/to/WorkBuddy.zip"
	@echo "  make run-app"
	@echo "Packaging:"
	@echo "  make deb"
	@echo "  make rpm"
	@echo "  make pacman"
	@echo "  make package"
	@echo "  make install"
	@echo "Release:"
	@echo "  make release-bundle VERSION=1.2.0   # build dist/release assets"
	@echo "  make release VERSION=1.2.0          # tag + push, CI publishes"
	@echo "  make release-local VERSION=1.2.0    # publish from this machine"
	@echo "Other:"
	@echo "  make check"
	@echo "  make clean"

deps:
	bash scripts/install-deps.sh

preflight:
	@bash scripts/preflight.sh $(DMG)

build-app:
	@if [ -n "$(DMG)" ]; then bash install.sh "$(DMG)"; else bash install.sh; fi

run-app:
	bash workbuddy-app/start.sh

deb:
	bash scripts/build-deb.sh

rpm:
	bash scripts/build-rpm.sh

pacman:
	bash scripts/build-pacman.sh

package:
	bash scripts/package.sh

install:
	bash scripts/install-package.sh

check:
	bash -n install.sh scripts/*.sh scripts/lib/*.sh
	node --check scripts/lib/apply-linux-patches.js

# Build the files attached to a GitHub release. Ships the toolchain, not a
# converted application: that payload is Tencent's proprietary app.asar.
release-bundle:
	@bash scripts/make-release.sh "$(VERSION)"

# Tag and push; .github/workflows/release.yml does the publishing.
release:
	@[ -n "$(VERSION)" ] || { echo "usage: make release VERSION=1.2.0" >&2; exit 2; }
	git tag -a "v$(VERSION)" -m "v$(VERSION)"
	git push origin "v$(VERSION)"

# Same thing without CI, using the local gh CLI.
release-local: release-bundle
	@[ -n "$(VERSION)" ] || { echo "usage: make release-local VERSION=1.2.0" >&2; exit 2; }
	gh release create "v$(VERSION)" \
		"dist/release/workbuddy-linux-$(VERSION).tar.gz" \
		dist/release/install-workbuddy.sh \
		dist/release/SHA256SUMS \
		--title "v$(VERSION)" \
		--notes-file dist/release/CHANGELOG.md \
		--latest

clean:
	rm -rf workbuddy-app workbuddy-app-next dist dist-next
	rm -rf .asar-tool .asar-extract .test-repack.asar .test-repack.asar.unpacked
	rm -f .tmp-asar-tool.js .tmp-asar-repack.js
