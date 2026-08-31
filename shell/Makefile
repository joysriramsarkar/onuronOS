# shell/Makefile — Build nilshell
CC ?= gcc
CFLAGS ?= -O2 -Wall -Wextra -I.

SRCS = nilshell.c gestures.c nil_softbus.c nilui_server.c convergence.c
OBJS = $(SRCS:.c=.o)

all: nilshell

nilshell: $(OBJS)
	$(CC) $(CFLAGS) -o $@ $(OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) nilshell
