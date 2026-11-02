# Build and check the project.
CC      := cc
CFLAGS  ?= -O2 -Wall
PREFIX   = /usr/local
OBJS     = main.o util.o

.PHONY: all build test lint clean install

all: build

build: $(OBJS)
	$(CC) $(CFLAGS) -o app $(OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

test: build
	./run-tests.sh

lint:
	cppcheck --quiet .

clean:
	rm -f app $(OBJS)

ifeq ($(shell uname),Darwin)
install: build
	install -m 755 app $(PREFIX)/bin
endif
