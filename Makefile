SHELL := /bin/bash

.PHONY: help snapshot
.DEFAULT_GOAL := help

help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  snapshot   - Generates a new file system snapshot."
	@echo "  help       - Shows this help message."

snapshot:
	@# Check if 'make' itself was invoked with sudo (SUDO_USER is set when sudo is used)
	@if [ -z "$$SUDO_USER" ]; then \
		echo "❌ Error: 'make snapshot' requires root privileges."; \
		echo "   Please run 'sudo make snapshot' instead."; \
		exit 1; \
	fi
	@echo "▶️  Generating snapshot..."
	@RUN_VIA_MAKE=true ./treesnap.sh
