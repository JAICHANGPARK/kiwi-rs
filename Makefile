KIWI_VERSION ?= latest
KIWI_PREFIX ?=
KIWI_MODEL_VARIANT ?= base

.PHONY: install-kiwi install-kiwi-win
install-kiwi:
	KIWI_PREFIX="$(KIWI_PREFIX)" KIWI_MODEL_VARIANT="$(KIWI_MODEL_VARIANT)" ./scripts/install_kiwi.sh "$(KIWI_VERSION)"

install-kiwi-win:
	powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install_kiwi.ps1 -Version "$(KIWI_VERSION)" -Prefix "$(KIWI_PREFIX)" -ModelVariant "$(KIWI_MODEL_VARIANT)"
