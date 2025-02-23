# Makefile для konpac

BIN_NAME := konpac
TARGET_DIR := ./target/release
INSTALL_DIR := /usr/bin
DATA_DIR := /var/lib/konpac
DB_FILE := $(DATA_DIR)/packages.db
PACKAGES_DIR := $(DATA_DIR)/packages

.DEFAULT_GOAL := build

build:
	@cargo build --release
	@echo "Сборка завершена. Для установки выполните: sudo make install"

install: build
	@if [ $$(id -u) -ne 0 ]; then \
		echo "Ошибка: Установка требует прав root. Запустите с sudo!"; \
		exit 1; \
	fi
	@install -Dm755 $(TARGET_DIR)/$(BIN_NAME) $(INSTALL_DIR)/$(BIN_NAME)
	@mkdir -p $(PACKAGES_DIR)
	@touch $(DB_FILE)
	@chmod 644 $(DB_FILE)
	@echo "Установка завершена!"

uninstall:
	@if [ $$(id -u) -ne 0 ]; then \
		echo "Ошибка: Удаление требует прав root. Запустите с sudo!"; \
		exit 1; \
	fi
	@rm -f $(INSTALL_DIR)/$(BIN_NAME)
	@rm -rf $(DATA_DIR)
	@echo "Удаление завершено"

clean:
	@cargo clean
	@echo "Очистка завершена"

.PHONY: build install uninstall clean